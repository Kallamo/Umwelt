// SPDX-License-Identifier: AGPL-3.0-only

use std::io::{self, BufRead, Write};
use std::sync::Arc;

mod bench_gen;

use serde_json::{Value, json};
use umwelt_api::{Durability, Engine, SqliteStore, SystemClock, UuidV7Gen};
use umwelt_protocol::{Dispatcher, ProtocolErrorCode, ResponseEnvelope, failure};

const MAX_LINE_BYTES: usize = 16 * 1024 * 1024;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let result = match arguments.first().map(String::as_str) {
        Some("serve") => serve_command(&arguments[1..]),
        Some("call") => call_command(&arguments[1..]),
        Some("create") => create_command(&arguments[1..]),
        Some("inspect") => inspect_command(&arguments[1..]),
        Some("bench-gen") => bench_gen::command(&arguments[1..]),
        Some("bench-stats") => bench_gen::stats_command(&arguments[1..]),
        Some("--version" | "-V" | "version") => {
            println!("{}", version_line());
            Ok(0)
        }
        _ => {
            eprintln!("Usage: umwelt <serve|call|create|inspect|bench-gen|--version> ...");
            return 1;
        }
    };
    match result {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            1
        }
    }
}

fn engine(store: SqliteStore) -> Engine<SqliteStore> {
    Engine::new(store, Arc::new(SystemClock), Arc::new(UuidV7Gen))
}

const SERVE_USAGE: &str = "Usage: umwelt serve <file.umw> [--create] [--durability normal|full]";

fn serve_command(arguments: &[String]) -> Result<i32, String> {
    let (path, options) = arguments
        .split_first()
        .ok_or_else(|| SERVE_USAGE.to_owned())?;
    let mut create = false;
    let mut durability = Durability::default();
    let mut options = options.iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--create" => create = true,
            "--durability" => {
                durability = match options.next().map(String::as_str) {
                    Some("normal") => Durability::Normal,
                    Some("full") => Durability::Full,
                    _ => return Err(SERVE_USAGE.to_owned()),
                }
            }
            _ => return Err(SERVE_USAGE.to_owned()),
        }
    }
    let store = if create {
        SqliteStore::create_with(path, durability)
    } else {
        SqliteStore::open_with(path, durability)
    }
    .map_err(|error| format!("{}: {}", error_code(&error), error.message))?;
    serve(engine(store))
}

fn serve(engine: Engine<SqliteStore>) -> Result<i32, String> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut dispatcher = Dispatcher::new(engine);
    loop {
        match read_bounded_line(&mut input)
            .map_err(|error| format!("stdin read failed: {error}"))?
        {
            InputLine::Eof => {
                if let Err(error) = dispatcher.shutdown() {
                    eprintln!("{}: {}", error.code.as_str(), error.message);
                }
                return Ok(0);
            }
            InputLine::TooLarge => {
                let response = failure(
                    None,
                    ProtocolErrorCode::PayloadTooLarge,
                    "Request line exceeds 16 MiB.",
                    None,
                );
                if write_response(&mut output, &response).is_err() {
                    eprintln!("stdout write failed");
                    return Ok(2);
                }
            }
            InputLine::Line(line) => {
                let response = dispatcher.handle_line(&line);
                if write_response(&mut output, &response).is_err() {
                    eprintln!("stdout write failed");
                    return Ok(2);
                }
                if dispatcher.shutdown_requested() {
                    return Ok(0);
                }
            }
        }
    }
}

fn call_command(arguments: &[String]) -> Result<i32, String> {
    if arguments.len() != 3 {
        return Err("Usage: umwelt call <file.umw> <op> <json>".to_owned());
    }
    let payload: Value = serde_json::from_str(&arguments[2])
        .map_err(|error| format!("Invalid payload JSON: {error}"))?;
    let store = SqliteStore::open(&arguments[0])
        .map_err(|error| format!("{}: {}", error_code(&error), error.message))?;
    let mut dispatcher = Dispatcher::new(engine(store));
    let hello = json!({"v":1,"id":"implicit-hello","op":"hello","payload":{"protocol":{"min":1,"max":1},"client":format!("umwelt-cli/{}", env!("CARGO_PKG_VERSION"))}});
    let _ = dispatcher.handle_line(&hello.to_string());
    let request = json!({"v":1,"id":"call","op":arguments[1],"payload":payload});
    let response = dispatcher.handle_line(&request.to_string());
    write_response(&mut io::stdout().lock(), &response)
        .map_err(|error| format!("stdout write failed: {error}"))?;
    Ok(if response.is_success() { 0 } else { 1 })
}

fn create_command(arguments: &[String]) -> Result<i32, String> {
    if arguments.len() != 1 {
        return Err("Usage: umwelt create <file.umw>".to_owned());
    }
    let store = SqliteStore::create(&arguments[0])
        .map_err(|error| format!("{}: {}", error_code(&error), error.message))?;
    drop(store);
    println!(
        "{}",
        json!({"created":arguments[0],"storage":umwelt_api::STORAGE_VERSION})
    );
    Ok(0)
}

fn inspect_command(arguments: &[String]) -> Result<i32, String> {
    if arguments.len() != 1 {
        return Err("Usage: umwelt inspect <file.umw>".to_owned());
    }
    let store = SqliteStore::open(&arguments[0])
        .map_err(|error| format!("{}: {}", error_code(&error), error.message))?;
    let engine = engine(store);
    let projects = engine
        .projects()
        .map_err(|error| format!("{}: {}", error_code(&error), error.message))?;
    println!(
        "{}",
        json!({"storage":umwelt_api::STORAGE_VERSION,"contract":umwelt_api::UMWELT_CONTRACT_VERSION,"projects":projects})
    );
    Ok(0)
}

/// The product version and the format versions a client or a file must match
/// (see spec/versioning.md).
fn version_line() -> String {
    format!(
        "umwelt {} (contract {}, protocol {}, storage {})",
        env!("CARGO_PKG_VERSION"),
        umwelt_api::UMWELT_CONTRACT_VERSION,
        umwelt_protocol::UMWELT_PROTOCOL_VERSION,
        umwelt_api::STORAGE_VERSION,
    )
}

fn error_code(error: &umwelt_core::UmweltError) -> String {
    serde_json::to_value(error.code)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "INTERNAL".to_owned())
}

fn write_response(output: &mut impl Write, response: &ResponseEnvelope) -> io::Result<()> {
    serde_json::to_writer(&mut *output, response)?;
    output.write_all(b"\n")?;
    output.flush()
}

enum InputLine {
    Eof,
    Line(String),
    TooLarge,
}

fn read_bounded_line(input: &mut impl BufRead) -> io::Result<InputLine> {
    let mut bytes = Vec::new();
    let mut oversized = false;
    loop {
        let (available, newline) = {
            let buffer = input.fill_buf()?;
            if buffer.is_empty() {
                if bytes.is_empty() && !oversized {
                    return Ok(InputLine::Eof);
                }
                return if oversized {
                    Ok(InputLine::TooLarge)
                } else {
                    Ok(InputLine::Line(
                        String::from_utf8_lossy(&bytes).into_owned(),
                    ))
                };
            }
            (buffer.len(), buffer.iter().position(|byte| *byte == b'\n'))
        };
        let consumed = newline.map_or(available, |position| position + 1);
        if !oversized {
            let buffer = input.fill_buf()?;
            let content_end = newline.unwrap_or(consumed);
            let remaining = MAX_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..content_end.min(remaining)]);
            if content_end > remaining {
                oversized = true;
            }
        }
        input.consume(consumed);
        if newline.is_some() {
            return if oversized {
                Ok(InputLine::TooLarge)
            } else {
                Ok(InputLine::Line(
                    String::from_utf8_lossy(&bytes)
                        .trim_end_matches('\r')
                        .to_owned(),
                ))
            };
        }
    }
}
