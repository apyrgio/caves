//! Errors
//!
//! In this module, we define the errors that `caves` can return, when it
//! encounters a problem. Each error applies to a different situation and has
//! a helpful display message, to make error handling easier for users of this
//! library.

use anyhow;
use thiserror;

/// Errors for every problem that `caves` may encounter.
///
/// Each enum variant should apply to a different error that `caves` may
/// encounter. Every variant has its own error message, which gives the
/// context for the error.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The key was not found.
    #[error("Key with name `{0}` was not found")]
    NotFound(String),

    // FIXME: Should I add more context for the error here?
    /// An internal error occurred.
    ///
    /// This usually means that a transient error occurred, or that there's a
    /// configuration error.
    #[error("An internal error occurred: {0}")]
    Internal(anyhow::Error),

    // FIXME: Should I add more context for the error here?
    /// An unexpected error occurred. This must be a bug on our side.
    #[error("An unexpected error occurred: {0}")]
    Bug(anyhow::Error),
}

// FIXME: It's ugly to define all of our errors here.
impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (Error::Bug(_), Error::Bug(_)) => true,
            (Error::Internal(_), Error::Internal(_)) => true,
            (Error::NotFound(s1), Error::NotFound(s2)) => s1 == s2,
            _ => false,
        }
    }
}

impl Error {
    /// Create an internal error from a string.
    pub fn internal_from_msg(msg: String) -> Self {
        let e = anyhow!(msg);
        Self::Internal(e)
    }
}
