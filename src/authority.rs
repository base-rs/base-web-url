// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::UrlError;
use crate::syntax::{self, is_registered_name_byte, is_userinfo_byte};
use std::fmt;
use std::net::Ipv6Addr;
use std::ops::Range;
use std::str::FromStr;

/// The authority of a URL: `[userinfo "@"] host [":" port]`, such as `example.com:8080`.
///
/// The host is a registered name or IPv4 address, such as `example.com` or `192.0.2.1`, or an IP literal in brackets, such as `[2001:db8::1]`. The authority keeps the spelling it was parsed from; hosts are compared exactly as written.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Authority {
    text: String,
    /// The position of the host in `text`, including any brackets.
    host: Range<usize>,
    port: Option<u16>,
}

impl Authority {
    /// Parse an authority, such as `user@example.com:8080`, following RFC 3986 section 3.2.
    ///
    /// # Errors
    ///
    /// [`UrlError::Authority`] for invalid user information or an invalid host, [`UrlError::Port`] for a port that is not a number from 0 to 65535, and [`UrlError::PercentEncoding`] for a malformed escape. An empty port, as in `example.com:`, is permitted and treated as absent.
    pub fn parse(text: &str) -> Result<Self, UrlError> {
        let host_start = match text.find('@') {
            Some(index) => {
                syntax::validate(&text[..index], is_userinfo_byte, UrlError::Authority)?;
                index + 1
            }
            None => 0,
        };
        let rest = &text[host_start..];
        let host_length = if rest.starts_with('[') {
            let end = rest.find(']').ok_or(UrlError::Authority)?;
            if !is_ip_literal(&rest[1..end]) {
                return Err(UrlError::Authority);
            }
            end + 1
        } else {
            let end = rest.find(':').unwrap_or(rest.len());
            syntax::validate(&rest[..end], is_registered_name_byte, UrlError::Authority)?;
            end
        };
        let port = match &rest[host_length..] {
            "" | ":" => None,
            port => {
                let digits = port.strip_prefix(':').ok_or(UrlError::Authority)?;
                if !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(UrlError::Port);
                }
                Some(digits.parse().map_err(|_| UrlError::Port)?)
            }
        };
        Ok(Self {
            text: text.to_owned(),
            host: host_start..host_start + host_length,
            port,
        })
    }

    /// The user information before `@`, still percent-encoded, such as `user` or `user:password`.
    pub fn userinfo(&self) -> Option<&str> {
        self.host.start.checked_sub(1).map(|end| &self.text[..end])
    }

    /// The host as written, still percent-encoded, with brackets around an IP literal, such as `example.com` or `[2001:db8::1]`. It may be empty, as in `file:///etc/hosts`.
    pub fn host(&self) -> &str {
        &self.text[self.host.clone()]
    }

    /// The port, if one is given.
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// The authority as written.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// `IP-literal` without its brackets: an `IPv6address` or `IPvFuture`, such as `v1.fe80::a`.
fn is_ip_literal(text: &str) -> bool {
    if let Some(future) = text.strip_prefix(['v', 'V']) {
        return match future.split_once('.') {
            Some((version, address)) => {
                !version.is_empty()
                    && version.bytes().all(|byte| byte.is_ascii_hexdigit())
                    && !address.is_empty()
                    && address.bytes().all(is_userinfo_byte)
            }
            None => false,
        };
    }
    text.parse::<Ipv6Addr>().is_ok()
}

impl FromStr for Authority {
    type Err = UrlError;

    fn from_str(text: &str) -> Result<Self, UrlError> {
        Self::parse(text)
    }
}

impl fmt::Display for Authority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl fmt::Debug for Authority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Authority")
            .field(&self.text)
            .finish()
    }
}

impl PartialOrd for Authority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Authority {
    /// Orders authorities by their text, from which the host and port are derived, so the order agrees with equality.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.text.cmp(&other.text)
    }
}
