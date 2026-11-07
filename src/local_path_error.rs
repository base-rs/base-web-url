// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::fmt;

/// Why a URL path could not be mapped to a local filesystem path, or a local path to a URL path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocalPathError {
    /// A path would leave the root directory: `..` would rise above it, or a local path does not begin with it.
    EscapesRoot,
    /// A URL path segment does not decode to exactly one file name, such as `%2F` or `%00`, or a file name cannot be a URL path segment, such as one containing NUL. On platforms other than Unix, both must also be Unicode.
    Component,
    /// A local path has a root or prefix, such as `/` or `C:`, so it cannot become a relative URL path.
    Absolute,
}

impl fmt::Display for LocalPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EscapesRoot => "path escapes the root directory",
            Self::Component => "path component does not correspond to one file name",
            Self::Absolute => "local path is not relative",
        })
    }
}

impl std::error::Error for LocalPathError {}
