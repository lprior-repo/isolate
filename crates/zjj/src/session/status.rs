//! Session status types and implementations

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use zjj_core::{Error, Result};

/// Session status representing the lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session is being created
    #[default]
    Creating,
    /// Session is active and ready for use
    Active,
    /// Session is temporarily paused
    Paused,
    /// Session work is completed
    Completed,
    /// Session creation or operation failed
    Failed,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Active => write!(f, "active"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl FromStr for SessionStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "creating" => Ok(Self::Creating),
            "active" => Ok(Self::Active),
            "paused" => Ok(Self::Paused),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(Error::validation_error(format!("Invalid status: {s}"))),
        }
    }
}
