// SPDX-License-Identifier: AGPL-3.0-only
use std::sync::atomic::{AtomicU64, Ordering};

use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use umwelt_core::{Clock, IdGen};

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_utc(&self) -> String {
        match OffsetDateTime::now_utc().format(&Rfc3339) {
            Ok(value) => value,
            Err(_) => "1970-01-01T00:00:00Z".to_owned(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UuidV7Gen;

impl IdGen for UuidV7Gen {
    fn new_id(&self) -> String {
        uuid::Uuid::now_v7().to_string()
    }
}

#[derive(Debug, Clone)]
pub struct FixedClock(pub String);

impl Clock for FixedClock {
    fn now_utc(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug)]
pub struct SeqIdGen {
    next: AtomicU64,
}

impl Default for SeqIdGen {
    fn default() -> Self {
        Self::new()
    }
}

impl SeqIdGen {
    pub const fn new() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }
}

impl IdGen for SeqIdGen {
    fn new_id(&self) -> String {
        let value = self.next.fetch_add(1, Ordering::Relaxed);
        format!("id-{value:06}")
    }
}
