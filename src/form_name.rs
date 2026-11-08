// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::{FormError, FormErrorKind, FormPath};
use std::borrow::Cow;
use std::fmt;

/// The complete, decoded name of a pair, borrowed from the input when it has no escapes.
#[derive(Clone)]
pub struct FormName<'a> {
    text: Cow<'a, str>,
    has_value: bool,
    offset: u64,
    depth_limit: usize,
}

impl<'a> FormName<'a> {
    pub(crate) fn new(
        text: Cow<'a, str>,
        has_value: bool,
        offset: u64,
        depth_limit: usize,
    ) -> Self {
        Self {
            text,
            has_value,
            offset,
            depth_limit,
        }
    }

    /// The decoded name.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The decoded name, borrowed from the input when it had no escapes.
    pub fn into_text(self) -> Cow<'a, str> {
        self.text
    }

    /// Whether the name is followed by `=` and so has a value, which may be empty, as in `a=`. A name without `=`, as in `a&b=1`, has none.
    pub fn has_value(&self) -> bool {
        self.has_value
    }

    /// The offset of the name in the input.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// The name read as a bracket path, such as `user[emails][]`, with at most the parser's [`depth_limit`](crate::FormLimits::depth_limit) segments.
    ///
    /// ```
    /// use base_web_url::{FormPathSegment, form_pairs};
    ///
    /// let pair = form_pairs(b"user[emails][]=ada%40example.com").next().unwrap()?;
    /// let path = pair.name().path()?;
    /// let segments: Vec<_> = path.segments().collect();
    /// assert_eq!(segments, [FormPathSegment::Key("user"), FormPathSegment::Key("emails"), FormPathSegment::Element]);
    /// # Ok::<(), base_web_url::FormError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`FormErrorKind::InvalidPath`] when the name is not `key` followed by any number of `[key]` and `[]`, where a key is not empty and contains neither `[` nor `]`, and [`FormErrorKind::DepthLimit`] for more segments than the limit. Either error has the name's offset.
    pub fn path(&self) -> Result<FormPath<'_>, FormError> {
        FormPath::parse(&self.text, self.depth_limit)
            .map_err(|kind: FormErrorKind| FormError::new(kind, self.offset))
    }
}

/// Names are equal when their text, whether they have a value, and their offsets are.
impl PartialEq for FormName<'_> {
    fn eq(&self, other: &Self) -> bool {
        (&self.text, self.has_value, self.offset) == (&other.text, other.has_value, other.offset)
    }
}

impl Eq for FormName<'_> {}

/// Shows the name's length rather than its text, since names must not reach logs.
impl fmt::Debug for FormName<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormName")
            .field("text", &format_args!("<{} bytes>", self.text.len()))
            .field("has_value", &self.has_value)
            .field("offset", &self.offset)
            .finish()
    }
}
