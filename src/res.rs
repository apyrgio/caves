//! Result type

use crate::errors;

/// A helper `Result` alias.
///
/// In order to make our function signatures shorter, we define this `Result`
/// alias so we don't have to repeat the following:
///
/// * The default return type, which is `Vec<u8>`.
/// * The default error type, which is [`errors::Error`].
///
/// [`errors::Error`]: ../errors/enum.Error.html
pub type Res = Result<Vec<u8>, errors::Error>;

/// Return an empty OK value.
pub fn empty_ok() -> Res {
    Ok(Vec::new())
}
