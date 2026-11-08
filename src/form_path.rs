// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::FormErrorKind;
use std::fmt;
use std::iter::FusedIterator;

/// A name read as a bracket path, such as `user[address][city]` or `tags[]`, as [`FormName::path`](crate::FormName::path) gives it.
///
/// The path is given as segments, so that a consumer can map them onto its own structures; it is not assembled into one.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FormPath<'a> {
    text: &'a str,
    depth: usize,
}

/// One segment of a [`FormPath`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FormPathSegment<'a> {
    /// A key, the name's first segment or the text of `[key]`.
    Key(&'a str),
    /// `[]`, an element of an array, whose position the consumer decides.
    Element,
}

/// The segments of a [`FormPath`], in order.
#[derive(Clone)]
pub struct FormPathSegments<'a> {
    rest: &'a str,
    first: bool,
}

impl<'a> FormPath<'a> {
    /// Read `text` as a bracket path, `key *( "[" [ key ] "]" )`, where a key is not empty and contains neither `[` nor `]`, with at most `depth_limit` segments, the first included.
    ///
    /// [`FormName::path`](crate::FormName::path) reads a pair's name with the parser's depth limit; this reads any other decoded name, such as a `multipart/form-data` field's, by the same rules.
    ///
    /// ```
    /// use base_web_url::{FormErrorKind, FormPath, FormPathSegment};
    ///
    /// let path = FormPath::parse("user[emails][]", 8)?;
    /// assert!(path.segments().eq([FormPathSegment::Key("user"), FormPathSegment::Key("emails"), FormPathSegment::Element]));
    /// assert_eq!(FormPath::parse("user[emails", 8), Err(FormErrorKind::InvalidPath));
    /// assert_eq!(FormPath::parse("a[b][c]", 2), Err(FormErrorKind::DepthLimit));
    /// # Ok::<(), FormErrorKind>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`FormErrorKind::InvalidPath`] when `text` is not a bracket path, and [`FormErrorKind::DepthLimit`] for more segments than the limit.
    pub fn parse(text: &'a str, depth_limit: usize) -> Result<Self, FormErrorKind> {
        let (first, mut rest) = text.split_at(text.find('[').unwrap_or(text.len()));
        if !is_key(first) {
            return Err(FormErrorKind::InvalidPath);
        }
        let mut depth = 1;
        while !rest.is_empty() {
            // The rest begins with `[`, followed by a key or nothing, and `]`.
            let close = rest[1..]
                .find(['[', ']'])
                .map(|index| index + 1)
                .filter(|&index| rest.as_bytes()[index] == b']')
                .ok_or(FormErrorKind::InvalidPath)?;
            rest = &rest[close + 1..];
            if !rest.is_empty() && !rest.starts_with('[') {
                return Err(FormErrorKind::InvalidPath);
            }
            depth += 1;
        }
        if depth > depth_limit {
            return Err(FormErrorKind::DepthLimit);
        }
        Ok(Self { text, depth })
    }

    /// The segments, in order. The first is always a key.
    pub fn segments(&self) -> FormPathSegments<'a> {
        FormPathSegments {
            rest: self.text,
            first: true,
        }
    }

    /// The number of segments, the first included, so `a[b][]` has a depth of 3.
    pub fn depth(&self) -> usize {
        self.depth
    }
}

fn is_key(text: &str) -> bool {
    !text.is_empty() && !text.contains(['[', ']'])
}

/// Shows the path's depth rather than its text, since names must not reach logs.
impl fmt::Debug for FormPath<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FormPath(<{} segments>)", self.depth)
    }
}

/// Shows a key's length rather than its text, since names must not reach logs.
impl fmt::Debug for FormPathSegment<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Key(key) => write!(formatter, "Key(<{} bytes>)", key.len()),
            Self::Element => formatter.write_str("Element"),
        }
    }
}

/// Shows the remaining length rather than the text, since names must not reach logs.
impl fmt::Debug for FormPathSegments<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FormPathSegments(<{} bytes>)", self.rest.len())
    }
}

impl<'a> Iterator for FormPathSegments<'a> {
    type Item = FormPathSegment<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // The path was validated, so the first segment runs to the first `[`, and each later one is `[` and a key or nothing, up to `]`.
        let segment = if self.first {
            self.first = false;
            let end = self.rest.find('[').unwrap_or(self.rest.len());
            let (key, rest) = self.rest.split_at(end);
            self.rest = rest;
            FormPathSegment::Key(key)
        } else {
            let close = self.rest.find(']')?;
            let key = &self.rest[1..close];
            self.rest = &self.rest[close + 1..];
            if key.is_empty() {
                FormPathSegment::Element
            } else {
                FormPathSegment::Key(key)
            }
        };
        Some(segment)
    }
}

impl FusedIterator for FormPathSegments<'_> {}
