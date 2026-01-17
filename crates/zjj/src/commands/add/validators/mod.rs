//! Validators for add command
//!
//! This module contains specialized validators for the add command,
//! each focused on a specific validation concern:
//! - `name`: Session name format and rules
//! - `exists`: Session existence in database
//! - `workspace`: JJ workspace availability
//! - `zellij`: Zellij running and accessible
//! - `dependencies`: Required commands installed

pub mod dependencies;
pub mod exists;
pub mod name;
pub mod workspace;
pub mod zellij;

pub use dependencies::validate_dependencies;
pub use exists::validate_not_exists;
pub use name::validate_session_name;
pub use workspace::validate_workspace_available;
pub use zellij::validate_zellij_running;
