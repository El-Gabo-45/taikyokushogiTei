//! Lightweight structured logging for the HTTP server.
//!
//! Every request is recorded with a correlation id (see `request_context` in
//! `super`). Entries go to stderr, to a bounded in-memory ring buffer that the
//! UI/ops can read via `GET /api/logs`, and optionally to a newline-delimited
//! JSON file (`TAIKYOKU_LOG_FILE`). No external logging crate is needed.

use serde::Serialize;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Severity of a log entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

/// One structured log entry (JSON-serialisable for `/api/logs` and the file).
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// Unix timestamp in milliseconds.
    pub ts_ms: u128,
    pub level: LogLevel,
    /// Correlation id from `X-Request-Id` (or the generated one).
    pub request_id: String,
    /// Short source tag, e.g. `http`, `search`, `session`.
    pub target: &'static str,
    pub message: String,
}

/// A bounded ring of recent entries plus an optional append-only JSONL file.
pub struct LogBuffer {
    entries: Mutex<VecDeque<LogEntry>>,
    capacity: usize,
    file: Option<Mutex<File>>,
}

impl LogBuffer {
    /// Create a buffer holding at most `capacity` entries. When `file` is set,
    /// every entry is also appended as one JSON line.
    pub fn new(capacity: usize, file: Option<&Path>) -> std::io::Result<Self> {
        let file = match file {
            Some(path) => Some(Mutex::new(
                OpenOptions::new().create(true).append(true).open(path)?,
            )),
            None => None,
        };
        Ok(Self {
            entries: Mutex::new(VecDeque::with_capacity(capacity.min(1024))),
            capacity: capacity.max(1),
            file,
        })
    }

    /// Record an entry. Never panics: a poisoned mutex or a failed file write
    /// degrades to stderr-only logging, which is always available.
    pub fn record(
        &self,
        level: LogLevel,
        request_id: &str,
        target: &'static str,
        message: impl Into<String>,
    ) {
        let entry = LogEntry {
            ts_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            level,
            request_id: request_id.to_owned(),
            target,
            message: message.into(),
        };

        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() == self.capacity {
                entries.pop_front();
            }
            entries.push_back(entry.clone());
        }

        eprintln!(
            "[{}] [{}] [{}] {}",
            entry.level.as_str(),
            entry.request_id,
            entry.target,
            entry.message
        );

        if let Some(file) = self.file.as_ref().and_then(|f| f.lock().ok()) {
            let mut file = file;
            if let Ok(line) = serde_json::to_string(&entry) {
                let _ = writeln!(file, "{line}");
                let _ = file.flush();
            }
        }
    }

    /// Shortcut for an error with no request id (background tasks).
    pub fn error(&self, target: &'static str, message: impl Into<String>) {
        self.record(LogLevel::Error, "-", target, message);
    }

    /// Shortcut for a warning with no request id (background tasks).
    pub fn warn(&self, target: &'static str, message: impl Into<String>) {
        self.record(LogLevel::Warn, "-", target, message);
    }

    /// Most recent entries, newest last, at most `limit`.
    pub fn recent(&self, limit: usize) -> Vec<LogEntry> {
        self.entries
            .lock()
            .map(|entries| {
                let skip = entries.len().saturating_sub(limit.min(self.capacity));
                entries.iter().skip(skip).cloned().collect()
            })
            .unwrap_or_default()
    }

    /// Configured ring capacity.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Whether an append-only JSONL file is configured.
    pub fn file_enabled(&self) -> bool {
        self.file.is_some()
    }

    /// Number of entries currently buffered.
    pub fn len(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
        }
    }

    /// Parse a level name case-insensitively (`error`, `warn`, `info`, `debug`).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "error" => Some(Self::Error),
            "warn" | "warning" => Some(Self::Warn),
            "info" => Some(Self::Info),
            "debug" => Some(Self::Debug),
            _ => None,
        }
    }
}
