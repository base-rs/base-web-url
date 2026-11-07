// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! The generic URI syntax of RFC 3986.

use crate::UrlError;

/// `unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"`.
pub(crate) const fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// `sub-delims = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="`.
pub(crate) const fn is_sub_delim(byte: u8) -> bool {
    matches!(
        byte,
        b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'='
    )
}

/// The literal characters of `pchar`, a path segment character.
pub(crate) const fn is_segment_byte(byte: u8) -> bool {
    is_unreserved(byte) || is_sub_delim(byte) || matches!(byte, b':' | b'@')
}

/// The literal characters of a path: segment characters and `/`.
pub(crate) const fn is_path_byte(byte: u8) -> bool {
    is_segment_byte(byte) || byte == b'/'
}

/// The literal characters of a query or fragment: `pchar / "/" / "?"`.
pub(crate) const fn is_query_byte(byte: u8) -> bool {
    is_path_byte(byte) || byte == b'?'
}

/// The literal characters of `userinfo`.
pub(crate) const fn is_userinfo_byte(byte: u8) -> bool {
    is_unreserved(byte) || is_sub_delim(byte) || byte == b':'
}

/// The literal characters of `reg-name`, which also covers IPv4 addresses.
pub(crate) const fn is_registered_name_byte(byte: u8) -> bool {
    is_unreserved(byte) || is_sub_delim(byte)
}

/// Check that `text` contains only bytes `allowed` accepts and well-formed percent-encodings.
pub(crate) fn validate(
    text: &str,
    allowed: fn(u8) -> bool,
    invalid: UrlError,
) -> Result<(), UrlError> {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if !bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit)
                || !bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit)
            {
                return Err(UrlError::PercentEncoding);
            }
            index += 3;
        } else if allowed(bytes[index]) {
            index += 1;
        } else {
            return Err(invalid);
        }
    }
    Ok(())
}

/// `scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`.
pub(crate) fn is_scheme(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

/// The value of an ASCII hexadecimal digit, which the caller has checked.
pub(crate) const fn hex_value(digit: u8) -> u8 {
    match digit {
        b'0'..=b'9' => digit - b'0',
        b'a'..=b'f' => digit - b'a' + 10,
        _ => digit - b'A' + 10,
    }
}

/// The components of a URI reference, split as in RFC 3986 Appendix B, before validation.
pub(crate) struct Parts<'a> {
    pub scheme: Option<&'a str>,
    pub authority: Option<&'a str>,
    pub path: &'a str,
    pub query: Option<&'a str>,
    pub fragment: Option<&'a str>,
}

/// Split a URI reference into its components: `[scheme ":"] ["//" authority] path ["?" query] ["#" fragment]`.
pub(crate) fn split(text: &str) -> Parts<'_> {
    let (rest, fragment) = match text.split_once('#') {
        Some((rest, fragment)) => (rest, Some(fragment)),
        None => (text, None),
    };
    let (rest, query) = match rest.split_once('?') {
        Some((rest, query)) => (rest, Some(query)),
        None => (rest, None),
    };
    // A scheme is everything before the first `:`, if that comes before any `/` and is not empty.
    let (scheme, rest) = match rest.find([':', '/']) {
        Some(index) if index > 0 && rest.as_bytes()[index] == b':' => {
            (Some(&rest[..index]), &rest[index + 1..])
        }
        _ => (None, rest),
    };
    let (authority, path) = match rest.strip_prefix("//") {
        Some(rest) => {
            let end = rest.find('/').unwrap_or(rest.len());
            (Some(&rest[..end]), &rest[end..])
        }
        None => (None, rest),
    };
    Parts {
        scheme,
        authority,
        path,
        query,
        fragment,
    }
}
