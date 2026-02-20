//! JSONL writer for streaming output
//!
//! This module provides the `JsonlWriter` type for writing JSONL output
//! to any `Write` implementation, and the `emit` function for stdout output.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::io::{self, Stdout, Write};

use serde::Serialize;

use super::OutputLine;
use crate::{Error, Result};

/// A writer for JSONL (JSON Lines) output.
///
/// Each call to `emit` writes a single JSON object followed by a newline.
/// The writer flushes after each line to ensure output is immediately available.
#[derive(Debug)]
pub struct JsonlWriter<W: Write> {
    writer: W,
}

impl<W: Write> JsonlWriter<W> {
    /// Create a new JsonlWriter wrapping the given writer.
    #[must_use]
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Emit a single OutputLine as JSON followed by a newline.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or if writing to the underlying
    /// writer fails.
    pub fn emit(&mut self, line: &OutputLine) -> Result<()> {
        let json = serde_json::to_string(line)
            .map_err(|e| Error::Serialization(format!("Failed to serialize output: {e}")))?;
        writeln!(self.writer, "{json}")
            .map_err(|e| Error::Io(format!("Failed to write output: {e}")))?;
        self.writer
            .flush()
            .map_err(|e| Error::Io(format!("Failed to flush output: {e}")))?;
        Ok(())
    }

    /// Emit any serializable value as JSON followed by a newline.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or if writing to the underlying
    /// writer fails.
    pub fn emit_value<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let json = serde_json::to_string(value)
            .map_err(|e| Error::Serialization(format!("Failed to serialize output: {e}")))?;
        writeln!(self.writer, "{json}")
            .map_err(|e| Error::Io(format!("Failed to write output: {e}")))?;
        self.writer
            .flush()
            .map_err(|e| Error::Io(format!("Failed to flush output: {e}")))?;
        Ok(())
    }
}

impl JsonlWriter<Stdout> {
    /// Create a JsonlWriter that writes to stdout.
    #[must_use]
    pub fn stdout() -> Self {
        Self::new(io::stdout())
    }
}

impl Default for JsonlWriter<Stdout> {
    fn default() -> Self {
        Self::stdout()
    }
}

/// Emit an OutputLine to stdout as a single JSON line.
///
/// This is a convenience function for the common case of writing to stdout.
/// Each call writes one JSON object followed by a newline and flushes.
///
/// # Errors
///
/// Returns an error if serialization fails or if writing to stdout fails.
pub fn emit(line: &OutputLine) -> Result<()> {
    JsonlWriter::stdout().emit(line)
}

/// Emit any serializable value to stdout as a single JSON line.
///
/// # Errors
///
/// Returns an error if serialization fails or if writing to stdout fails.
pub fn emit_value<T: Serialize>(value: &T) -> Result<()> {
    JsonlWriter::stdout().emit_value(value)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::output::{Context, SessionState};

    #[test]
    fn test_jsonl_writer_emit_writes_valid_json_line() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = JsonlWriter::new(&mut cursor);
            let line = OutputLine::session("test", SessionState::Active, 5);
            let result = writer.emit(&line);
            assert!(result.is_ok());
        }
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        assert!(output.contains(r#""type":"session"#));
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_jsonl_writer_emit_adds_newline() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = JsonlWriter::new(&mut cursor);
            let line = OutputLine::context("Test message");
            let result = writer.emit(&line);
            assert!(result.is_ok());
        }
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        assert!(output.ends_with('\n'));
        // Should be exactly one line
        assert_eq!(output.lines().count(), 1);
    }

    #[test]
    fn test_jsonl_writer_multiple_emit_calls_produce_multiple_lines() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = JsonlWriter::new(&mut cursor);
            let line1 = OutputLine::session("session1", SessionState::Active, 1);
            let line2 = OutputLine::session("session2", SessionState::Active, 2);
            let line3 = OutputLine::context("Done");
            assert!(writer.emit(&line1).is_ok());
            assert!(writer.emit(&line2).is_ok());
            assert!(writer.emit(&line3).is_ok());
        }
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains(r#""type":"session"#));
        assert!(lines[1].contains(r#""type":"session"#));
        assert!(lines[2].contains(r#""type":"context"#));
    }

    #[test]
    fn test_output_can_be_parsed_by_json_lines_decoder() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = JsonlWriter::new(&mut cursor);
            let line = OutputLine::session("test", SessionState::Active, 5);
            writer.emit(&line).unwrap();
        }
        let output = String::from_utf8(cursor.into_inner()).unwrap();

        // Each line should be parseable as JSON
        for line in output.lines() {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.get("type").is_some());
        }
    }

    #[test]
    fn test_emit_value_with_arbitrary_struct() {
        #[derive(Serialize)]
        struct TestStruct {
            name: String,
            count: usize,
        }

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = JsonlWriter::new(&mut cursor);
            let value = TestStruct {
                name: "test".to_string(),
                count: 42,
            };
            writer.emit_value(&value).unwrap();
        }
        let output = String::from_utf8(cursor.into_inner()).unwrap();
        assert!(output.contains(r#""name":"test"#));
        assert!(output.contains(r#""count":42"#));
    }

    #[test]
    fn test_jsonl_writer_default_is_stdout() {
        let _writer = JsonlWriter::default();
        // Just verify it doesn't panic
    }
}
