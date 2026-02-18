//! JSONL writer for streaming output

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use std::io::{self, Stdout, Write};

use serde_json;

use super::OutputLine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonlConfig {
    pub pretty: bool,
    pub flush_on_emit: bool,
}

impl Default for JsonlConfig {
    fn default() -> Self {
        Self {
            pretty: false,
            flush_on_emit: true,
        }
    }
}

impl JsonlConfig {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            pretty: false,
            flush_on_emit: true,
        }
    }

    #[must_use]
    pub const fn with_pretty(self, pretty: bool) -> Self {
        Self { pretty, ..self }
    }

    #[must_use]
    pub const fn with_flush_on_emit(self, flush_on_emit: bool) -> Self {
        Self {
            flush_on_emit,
            ..self
        }
    }
}

pub struct JsonlWriter<W> {
    writer: W,
    config: JsonlConfig,
}

impl<W: Write> JsonlWriter<W> {
    #[must_use]
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            config: JsonlConfig::default(),
        }
    }

    #[must_use]
    pub const fn with_config(writer: W, config: JsonlConfig) -> Self {
        Self { writer, config }
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn emit(&mut self, line: &OutputLine) -> io::Result<()> {
        let json = if self.config.pretty {
            serde_json::to_string_pretty(line)
        } else {
            serde_json::to_string(line)
        }
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        writeln!(self.writer, "{json}")?;

        if self.config.flush_on_emit {
            self.writer.flush()?;
        }

        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn emit_all<'a, I>(&mut self, lines: I) -> io::Result<()>
    where
        I: IntoIterator<Item = &'a OutputLine>,
    {
        lines.into_iter().try_for_each(|line| self.emit(line))
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    #[must_use]
    pub fn into_inner(self) -> W {
        self.writer
    }
}

impl JsonlWriter<Stdout> {
    #[must_use]
    pub fn stdout() -> Self {
        Self::new(io::stdout())
    }

    #[must_use]
    pub fn stdout_with_config(config: JsonlConfig) -> Self {
        Self::with_config(io::stdout(), config)
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn emit<W: Write>(writer: &mut W, line: &OutputLine) -> io::Result<()> {
    let json =
        serde_json::to_string(line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    writeln!(writer, "{json}")?;
    writer.flush()
}

#[allow(clippy::missing_errors_doc)]
pub fn emit_stdout(line: &OutputLine) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    emit(&mut handle, line)
}

#[allow(clippy::missing_errors_doc)]
pub fn emit_all_stdout<'a, I>(lines: I) -> io::Result<()>
where
    I: IntoIterator<Item = &'a OutputLine>,
{
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    lines
        .into_iter()
        .try_for_each(|line| emit(&mut handle, line))
}
