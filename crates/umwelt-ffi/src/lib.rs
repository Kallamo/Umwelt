// SPDX-License-Identifier: AGPL-3.0-only
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![allow(unknown_lints)]
#![allow(linker_messages)]

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;
use std::str;
use std::sync::Arc;

use serde_json::json;
use umwelt_api::{Durability, Engine, SqliteStore, SystemClock, UuidV7Gen};
use umwelt_protocol::Dispatcher;

const STATUS_OK: i32 = 0;
const STATUS_INVALID_ARGUMENT: i32 = 1;
const STATUS_OPEN_FAILED: i32 = 2;
const STATUS_POISONED: i32 = 3;
const STATUS_PANIC: i32 = 4;

#[repr(C)]
pub struct umwelt_buf {
    pub ptr: *mut u8,
    pub len: usize,
}

impl umwelt_buf {
    const fn empty() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }
}

enum HandleState {
    Ready(Box<Dispatcher<SqliteStore>>),
    Failed,
}

#[allow(non_camel_case_types)]
pub struct umwelt_engine {
    state: HandleState,
    poisoned: bool,
    last_error: Option<Vec<u8>>,
}

impl umwelt_engine {
    fn ready(store: SqliteStore) -> Self {
        let engine = Engine::new(store, Arc::new(SystemClock), Arc::new(UuidV7Gen));
        Self {
            state: HandleState::Ready(Box::new(Dispatcher::new(engine))),
            poisoned: false,
            last_error: None,
        }
    }

    fn failed(code: &str, message: &str) -> Self {
        Self {
            state: HandleState::Failed,
            poisoned: false,
            last_error: Some(error_json(code, message)),
        }
    }

    fn set_error(&mut self, code: &str, message: &str) {
        self.last_error = Some(error_json(code, message));
    }
}

fn error_json(code: &str, message: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({ "code": code, "message": message })).unwrap_or_else(|_| {
        b"{\"code\":\"PANIC\",\"message\":\"Could not serialize FFI error.\"}".to_vec()
    })
}

fn write_buf(out: *mut umwelt_buf, bytes: Vec<u8>) -> i32 {
    if out.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = if len == 0 {
        ptr::null_mut()
    } else {
        Box::into_raw(boxed).cast::<u8>()
    };
    unsafe {
        *out = umwelt_buf { ptr, len };
    }
    STATUS_OK
}

fn input<'a>(pointer: *const u8, len: usize) -> Result<&'a str, &'static str> {
    if pointer.is_null() {
        return Err("Pointer must not be null.");
    }
    let bytes = unsafe { slice::from_raw_parts(pointer, len) };
    str::from_utf8(bytes).map_err(|_| "Input must be valid UTF-8.")
}

fn poison(handle: *mut umwelt_engine, message: &str) {
    if let Some(value) = unsafe { handle.as_mut() } {
        value.poisoned = true;
        value.set_error("PANIC", message);
    }
}

#[unsafe(no_mangle)]
/// Returns the binary ABI version, currently 1.
pub extern "C" fn umwelt_abi_version() -> u32 {
    catch_unwind(|| 1_u32).unwrap_or(STATUS_PANIC as u32)
}

#[unsafe(no_mangle)]
/// Opens or creates a UTF-8 path without requiring a trailing NUL.
/// On OPEN_FAILED, `out` receives a failed handle queryable through `umwelt_last_error`.
///
/// # Safety
/// Pointer arguments must be valid for their declared lengths and writable where applicable.
pub unsafe extern "C" fn umwelt_open(
    path: *const u8,
    path_len: usize,
    create: i32,
    out: *mut *mut umwelt_engine,
) -> i32 {
    unsafe { umwelt_open_with_options(path, path_len, create, UMWELT_DURABILITY_NORMAL, out) }
}

/// `synchronous=NORMAL`: survives an application crash; a power loss may drop
/// the most recent commits. Never corrupts the file. Default of `umwelt_open`.
pub const UMWELT_DURABILITY_NORMAL: i32 = 0;
/// `synchronous=FULL`: every commit is flushed before it returns.
pub const UMWELT_DURABILITY_FULL: i32 = 1;

#[unsafe(no_mangle)]
/// Like `umwelt_open`, with an explicit durability
/// (`UMWELT_DURABILITY_NORMAL` or `UMWELT_DURABILITY_FULL`).
/// Any other durability value returns INVALID_ARGUMENT with a failed handle.
///
/// # Safety
/// Pointer arguments must be valid for their declared lengths and writable where applicable.
pub unsafe extern "C" fn umwelt_open_with_options(
    path: *const u8,
    path_len: usize,
    create: i32,
    durability: i32,
    out: *mut *mut umwelt_engine,
) -> i32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        unsafe {
            *out = ptr::null_mut();
        }
        let path = match input(path, path_len) {
            Ok(value) => value,
            Err(message) => {
                let handle = Box::new(umwelt_engine::failed("INVALID_ARGUMENT", message));
                unsafe {
                    *out = Box::into_raw(handle);
                }
                return STATUS_INVALID_ARGUMENT;
            }
        };
        let durability = match durability {
            UMWELT_DURABILITY_NORMAL => Durability::Normal,
            UMWELT_DURABILITY_FULL => Durability::Full,
            _ => {
                let handle = Box::new(umwelt_engine::failed(
                    "INVALID_ARGUMENT",
                    "Durability must be UMWELT_DURABILITY_NORMAL or UMWELT_DURABILITY_FULL.",
                ));
                unsafe {
                    *out = Box::into_raw(handle);
                }
                return STATUS_INVALID_ARGUMENT;
            }
        };
        let result = if create == 0 {
            SqliteStore::open_with(path, durability)
        } else {
            SqliteStore::create_with(path, durability)
        };
        match result {
            Ok(store) => {
                unsafe {
                    *out = Box::into_raw(Box::new(umwelt_engine::ready(store)));
                }
                STATUS_OK
            }
            Err(error) => {
                let code = serde_json::to_value(error.code)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "INTERNAL".to_owned());
                unsafe {
                    *out = Box::into_raw(Box::new(umwelt_engine::failed(&code, &error.message)));
                }
                STATUS_OPEN_FAILED
            }
        }
    })) {
        Ok(status) => status,
        Err(_) => {
            if !out.is_null() {
                unsafe {
                    *out = Box::into_raw(Box::new(umwelt_engine {
                        state: HandleState::Failed,
                        poisoned: true,
                        last_error: Some(error_json("PANIC", "Panic while opening Umwelt engine.")),
                    }));
                }
            }
            STATUS_PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Dispatches one JSON protocol v1 request and returns its JSON envelope.
/// The first request must be `hello`, exactly as with `umwelt serve`.
///
/// # Safety
/// `handle` must be live, `request` valid for `request_len`, and `out` writable.
pub unsafe extern "C" fn umwelt_call(
    handle: *mut umwelt_engine,
    request: *const u8,
    request_len: usize,
    out: *mut umwelt_buf,
) -> i32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        let engine = unsafe { &mut *handle };
        if out.is_null() {
            engine.set_error("INVALID_ARGUMENT", "Output pointer must not be null.");
            return STATUS_INVALID_ARGUMENT;
        }
        unsafe {
            *out = umwelt_buf::empty();
        }
        if engine.poisoned {
            engine.set_error("POISONED", "The Umwelt handle is poisoned.");
            return STATUS_POISONED;
        }
        let request = match input(request, request_len) {
            Ok(value) => value,
            Err(message) => {
                engine.set_error("INVALID_ARGUMENT", message);
                return STATUS_INVALID_ARGUMENT;
            }
        };
        #[cfg(feature = "test-panic")]
        if serde_json::from_str::<serde_json::Value>(request)
            .ok()
            .and_then(|value| {
                value
                    .get("op")
                    .and_then(|op| op.as_str())
                    .map(str::to_owned)
            })
            .as_deref()
            == Some("ffi.testPanic")
        {
            panic!("diagnostic FFI panic");
        }
        match &mut engine.state {
            HandleState::Ready(dispatcher) => {
                match serde_json::to_vec(&dispatcher.handle_line(request)) {
                    Ok(response) => write_buf(out, response),
                    Err(error) => {
                        engine.set_error(
                            "INTERNAL",
                            &format!("Could not serialize protocol response: {error}"),
                        );
                        STATUS_PANIC
                    }
                }
            }
            HandleState::Failed => STATUS_OPEN_FAILED,
        }
    })) {
        Ok(status) => status,
        Err(_) => {
            poison(handle, "Panic while processing Umwelt request.");
            STATUS_PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Copies the last nonzero-status error as JSON into `out`.
///
/// # Safety
/// `handle` must be live and `out` must be writable.
pub unsafe extern "C" fn umwelt_last_error(
    handle: *mut umwelt_engine,
    out: *mut umwelt_buf,
) -> i32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() || out.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        unsafe {
            *out = umwelt_buf::empty();
        }
        let engine = unsafe { &mut *handle };
        write_buf(out, engine.last_error.clone().unwrap_or_default())
    })) {
        Ok(status) => status,
        Err(_) => {
            poison(handle, "Panic while reading Umwelt error.");
            STATUS_PANIC
        }
    }
}

#[unsafe(no_mangle)]
/// Releases a buffer allocated by this library. A null empty buffer is accepted.
///
/// # Safety
/// A nonempty buffer must have been returned by this library and not previously freed.
pub unsafe extern "C" fn umwelt_buf_free(buffer: umwelt_buf) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !buffer.ptr.is_null() && buffer.len != 0 {
            let raw = ptr::slice_from_raw_parts_mut(buffer.ptr, buffer.len);
            unsafe {
                drop(Box::from_raw(raw));
            }
        }
    }));
}

#[unsafe(no_mangle)]
/// Checkpoints the WAL when possible and releases the handle. NULL is accepted.
///
/// # Safety
/// A non-NULL handle must have been returned by `umwelt_open` and not previously closed.
pub unsafe extern "C" fn umwelt_close(handle: *mut umwelt_engine) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() {
            return;
        }
        let mut engine = unsafe { Box::from_raw(handle) };
        if let HandleState::Ready(dispatcher) = &mut engine.state {
            let _ = dispatcher.shutdown();
        }
    }));
}
