// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::fmt;

/// Why a URL, URL reference, path, or component was rejected.
///
/// Errors do not repeat the rejected input, which may be large or untrusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum UrlError {
    /// A character is not permitted where it appears by the generic syntax of RFC 3986, such as a space, `|`, or non-ASCII text that is not percent-encoded.
    Syntax,
    /// The scheme does not begin with a letter followed by letters, digits, `+`, `-`, or `.`.
    Scheme,
    /// The authority's user information or host is not valid, such as a malformed IPv6 address.
    Host,
    /// The port is not a number from 0 to 65535.
    Port,
    /// A `%` is not followed by two hexadecimal digits.
    PercentEncoding,
    /// Percent-decoded bytes are not UTF-8.
    Utf8,
    /// A URL requires a scheme, but the reference has none.
    Relative,
    /// The path cannot be combined with the other components: a path after an authority must be empty or begin with `/`, a path without an authority cannot begin with `//`, and the first segment of a reference without a scheme cannot contain `:`.
    Path,
}

impl fmt::Display for UrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Syntax => "invalid character in URL",
            Self::Scheme => "invalid URL scheme",
            Self::Host => "invalid URL authority",
            Self::Port => "invalid URL port",
            Self::PercentEncoding => "invalid percent-encoding in URL",
            Self::Utf8 => "percent-decoded URL component is not UTF-8",
            Self::Relative => "URL has no scheme",
            Self::Path => "URL path is incompatible with its other components",
        })
    }
}

impl std::error::Error for UrlError {}
