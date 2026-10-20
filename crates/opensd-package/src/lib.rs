//! The `.opensd` plugin package format (format version 1, SDK 1).
//!
//! A package is a plain ZIP archive: `manifest.json`, `index.json` with the
//! size and SHA-256 of every other file, the plugin code (`plugin.wasm` or
//! `plugin.js`) and its resources. Nothing in it is a native executable.
//! The format is described in the [package reference](https://wiki.opensd.dev/docs/sdk/package).

macro_rules! bail {
    ($($arg:tt)*) => { return Err($crate::Error(format!($($arg)*))) };
}

macro_rules! ensure {
    ($cond:expr, $($arg:tt)*) => { if !$cond { bail!($($arg)*); } };
}

mod manifest;
mod package;
mod paths;
pub mod wasm;

pub use manifest::*;
pub use package::*;
pub use paths::safe_path;

/// Major SDK and protocol version a manifest declares with `sdk = 1`.
pub const SDK: u32 = 1;
/// Version of the archive layout in `index.json`.
pub const FORMAT_VERSION: u32 = 1;
/// Package file extension, without the dot.
pub const EXTENSION: &str = "opensd";

/// A validation or I/O failure, with a message meant for the plugin author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error(e.to_string())
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(e: zip::result::ZipError) -> Self {
        Error(format!("Invalid archive: {e}"))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
