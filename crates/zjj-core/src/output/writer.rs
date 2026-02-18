//! JSONL writer for streaming output

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::io::{self, Write};

use serde_json;

use super::OutputLine;

pub struct JsonlWriter<W> {
    writer: W,
}

impl<W: Write> JsonlWriter<W> {
    /// Create a new `JsonlWriter` wrapping the given writer.
    pub const fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Emit a single `OutputLine` as JSONL to the wrapped writer.
    ///
    /// # Errors
    /// Returns an error if serialization fails or writing to the underlying writer fails.
    pub fn emit(&mut self, line: &OutputLine) -> io::Result<()> {
        let json = serde_json::to_string(line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{json}")
    }

    /// Emit multiple `OutputLine` variants as JSONL to the wrapped writer.
    ///
    /// # Errors
    /// Returns an error if serialization fails or writing to the underlying writer fails.
    pub fn emit_all<'a, I>(&mut self, lines: I) -> io::Result<()>
    where
        I: IntoIterator<Item = &'a OutputLine>,
    {
        lines.into_iter().try_for_each(|line| self.emit(line))
    }

    /// Flush the underlying writer.
    ///
    /// # Errors
    /// Returns an error if flushing the underlying writer fails.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

/// Emit a single `OutputLine` as JSONL to the given writer.
///
/// # Errors
/// Returns an error if serialization fails or writing to the writer fails.
pub fn emit<W: Write>(writer: &mut W, line: &OutputLine) -> io::Result<()> {
    let json =
        serde_json::to_string(line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(writer, "{json}")
}
