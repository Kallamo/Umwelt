// SPDX-License-Identifier: AGPL-3.0-only
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UmweltErrorCode {
    ValidationFailed,
    NotFound,
    ProjectIsolationViolation,
    Conflict,
    VersionConflict,
    AttemptFailed,
    InvalidState,
    DatabaseError,
    IntegrityCheckFailed,
    ImportFailed,
    NotUmweltFile,
    UnsupportedFileVersion,
    FileLocked,
}

#[derive(Debug, Error)]
#[error("{message}")]
pub struct UmweltError {
    pub code: UmweltErrorCode,
    pub message: String,
    pub details: Option<Value>,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl UmweltError {
    pub fn public(code: UmweltErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            source: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn internal(
        code: UmweltErrorCode,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
            source: Some(Box::new(source)),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::public(UmweltErrorCode::ValidationFailed, message)
    }

    pub fn internal_for_cli(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::internal(UmweltErrorCode::ImportFailed, message, source)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::public(UmweltErrorCode::NotFound, message)
    }
}

impl From<serde_json::Error> for UmweltError {
    fn from(source: serde_json::Error) -> Self {
        Self::internal(
            UmweltErrorCode::ValidationFailed,
            "Stored Umwelt data is invalid.",
            source,
        )
    }
}

pub type Result<T> = std::result::Result<T, UmweltError>;
