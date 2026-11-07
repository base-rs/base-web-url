// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::UrlError;
use crate::syntax::{hex_value, is_unreserved};
use std::borrow::Cow;
use std::fmt::Write as _;

/// Percent-encode `value` for use as one URL component, such as a path segment or a query parameter value.
///
/// Every byte except the unreserved characters `A`–`Z`, `a`–`z`, `0`–`9`, `-`, `.`, `_`, and `~` is encoded as `%XX` with uppercase hexadecimal digits, so the result never contains a delimiter such as `/`, `?`, `&`, or `=`. Text is encoded as UTF-8.
///
/// ```
/// use base_web_url::encode_component;
///
/// assert_eq!(encode_component("café & bar/baz"), "caf%C3%A9%20%26%20bar%2Fbaz");
/// ```
pub fn encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if is_unreserved(byte) {
            encoded.push(char::from(byte));
        } else {
            write!(encoded, "%{byte:02X}").expect("writing to a string cannot fail");
        }
    }
    encoded
}

/// Decode the `%XX` escapes in `text` and interpret the result as UTF-8.
///
/// Other characters are kept as they are; `+` is not a space here, as it is only in form content. Text without escapes is borrowed.
///
/// ```
/// use base_web_url::decode_component;
///
/// assert_eq!(decode_component("caf%C3%A9%20bar")?, "café bar");
/// # Ok::<(), base_web_url::UrlError>(())
/// ```
///
/// # Errors
///
/// [`UrlError::PercentEncoding`] for a `%` not followed by two hexadecimal digits, and [`UrlError::Utf8`] when the decoded bytes are not UTF-8.
pub fn decode_component(text: &str) -> Result<Cow<'_, str>, UrlError> {
    if !text.contains('%') {
        return Ok(Cow::Borrowed(text));
    }
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            match (bytes.get(index + 1), bytes.get(index + 2)) {
                (Some(&high), Some(&low))
                    if high.is_ascii_hexdigit() && low.is_ascii_hexdigit() =>
                {
                    decoded.push(hex_value(high) << 4 | hex_value(low))
                }
                _ => return Err(UrlError::PercentEncoding),
            }
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded)
        .map(Cow::Owned)
        .map_err(|_| UrlError::Utf8)
}
