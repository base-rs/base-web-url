// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::fmt;

/// Why a URL path could not be mapped to a local filesystem path, or a local path to a URL path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocalPathError {
    /// `..` segments would leave the root directory.
    EscapesRoot,
    /// A segment does not correspond to exactly one file name: once decoded, it is not UTF-8, or contains NUL or a path separator, such as `%2F`. Or a local path component is not UTF-8.
    Component,
    /// A local path has a root or prefix, such as `/` or `C:`, so it cannot become a relative URL path.
    Absolute,
}

impl fmt::Display for LocalPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EscapesRoot => "URL path escapes the root directory",
            Self::Component => "URL path segment does not correspond to one file name",
            Self::Absolute => "local path is not relative",
        })
    }
}

impl std::error::Error for LocalPathError {}
