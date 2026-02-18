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
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn emit(&mut self, line: &OutputLine) -> io::Result<()> {
        let json = serde_json::to_string(line)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{}", json)
    }

    pub fn emit_all<'a, I>(&mut self, lines: I) -> io::Result<()>
    where
        I: IntoIterator<Item = &'a OutputLine>,
    {
        lines.into_iter().try_for_each(|line| self.emit(line))
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

pub fn emit<W: Write>(writer: &mut W, line: &OutputLine) -> io::Result<()> {
    let json =
        serde_json::to_string(line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(writer, "{}", json)
}
