// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::syntax::{self, hex_value, is_path_byte, is_segment_byte, is_unreserved};
use crate::{LocalPathError, UrlError, decode_component, encode_component};
use std::borrow::Cow;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

/// A URL path, kept in its percent-encoded form so that segment boundaries are never lost.
///
/// A literal `/` separates segments, while `%2F` is data within one segment. [`Self::segments`] gives the encoded segments, and [`Self::components`] decodes them; build a path from decoded application values with [`Self::from_components`], which encodes each value as one segment.
///
/// ```
/// use base_web_url::UrlPath;
///
/// let path = UrlPath::from_components(["", "docs", "a/b c"]);
/// assert_eq!(path.as_str(), "/docs/a%2Fb%20c");
/// assert_eq!(path.segments().collect::<Vec<_>>(), ["", "docs", "a%2Fb%20c"]);
/// assert_eq!(path.components()?, ["", "docs", "a/b c"]);
/// # Ok::<(), base_web_url::UrlError>(())
/// ```
///
/// Paths compare by their exact encoded spelling; use [`Self::normalize`] and [`Self::simplify`] first to compare equivalent spellings.
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UrlPath {
    encoded: String,
}

/// A dot segment: `.` or `..`.
#[derive(Clone, Copy, PartialEq)]
enum DotSegment {
    Current,
    Parent,
}

/// Characters that cannot appear in one local file name.
#[cfg(not(windows))]
const RESERVED_FILE_NAME_CHARACTERS: &[char] = &['\0', '/'];
/// Characters that cannot appear in one local file name. On Windows, `:` would name a drive or an alternate data stream.
#[cfg(windows)]
const RESERVED_FILE_NAME_CHARACTERS: &[char] = &['\0', '/', '\\', ':'];

impl UrlPath {
    /// The empty path.
    pub const fn new() -> Self {
        Self {
            encoded: String::new(),
        }
    }

    /// Parse a percent-encoded path, such as `/docs/a%20b`.
    ///
    /// # Errors
    ///
    /// [`UrlError::Syntax`] for a character outside the path grammar of RFC 3986, such as a space, `?`, or `#`, and [`UrlError::PercentEncoding`] for a malformed escape.
    pub fn parse(text: &str) -> Result<Self, UrlError> {
        syntax::validate(text, is_path_byte, UrlError::Syntax)?;
        Ok(Self {
            encoded: text.to_owned(),
        })
    }

    /// A path from percent-encoded segments, joined with `/`. A first segment that is empty makes the path absolute, and a last one that is empty gives it a trailing `/`.
    ///
    /// # Errors
    ///
    /// As for [`Self::parse`]; a literal `/` within a segment is a [`UrlError::Syntax`] error.
    pub fn from_segments<S: AsRef<str>>(
        segments: impl IntoIterator<Item = S>,
    ) -> Result<Self, UrlError> {
        let mut encoded = String::new();
        for (index, segment) in segments.into_iter().enumerate() {
            let segment = segment.as_ref();
            syntax::validate(segment, is_segment_byte, UrlError::Syntax)?;
            if index > 0 {
                encoded.push('/');
            }
            encoded.push_str(segment);
        }
        Ok(Self { encoded })
    }

    /// A path from decoded components, each percent-encoded as one segment, so a `/` within a component stays data. A first component that is empty makes the path absolute, and a last one that is empty gives it a trailing `/`.
    pub fn from_components<C: AsRef<str>>(components: impl IntoIterator<Item = C>) -> Self {
        let segments: Vec<String> = components
            .into_iter()
            .map(|component| encode_component(component.as_ref()))
            .collect();
        Self {
            encoded: segments.join("/"),
        }
    }

    /// A relative URL path from a relative local path, such as `docs/My Report.pdf` to `docs/My%20Report.pdf`, encoding each file name as one segment.
    ///
    /// `.` components are dropped, and `..` components are kept. This is the inverse of [`Self::local_path`] for paths beneath a root.
    ///
    /// # Errors
    ///
    /// [`LocalPathError::Absolute`] for a path with a root or prefix, such as `/srv` or `C:\srv`, and [`LocalPathError::Component`] for a file name that is not UTF-8.
    pub fn from_local_path(path: impl AsRef<Path>) -> Result<Self, LocalPathError> {
        let mut segments = Vec::new();
        for component in path.as_ref().components() {
            match component {
                Component::Normal(name) => segments.push(encode_component(
                    name.to_str().ok_or(LocalPathError::Component)?,
                )),
                Component::CurDir => {}
                Component::ParentDir => segments.push("..".to_owned()),
                Component::RootDir | Component::Prefix(_) => return Err(LocalPathError::Absolute),
            }
        }
        Ok(Self {
            encoded: segments.join("/"),
        })
    }

    /// The percent-encoded path.
    pub fn as_str(&self) -> &str {
        &self.encoded
    }

    /// Whether the path has no segments.
    pub fn is_empty(&self) -> bool {
        self.encoded.is_empty()
    }

    /// Whether the path begins with `/`.
    pub fn is_absolute(&self) -> bool {
        self.encoded.starts_with('/')
    }

    /// Whether the path ends with `/`, naming a directory rather than a file.
    pub fn is_directory(&self) -> bool {
        self.encoded.ends_with('/')
    }

    /// The percent-encoded segments between `/` separators. An absolute path's first segment, and a directory path's last, are empty; the empty path has no segments.
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        let count = if self.encoded.is_empty() {
            0
        } else {
            usize::MAX
        };
        self.encoded.split('/').take(count)
    }

    /// The decoded segments, as for [`Self::segments`].
    ///
    /// # Errors
    ///
    /// [`UrlError::Utf8`] when a segment does not decode to UTF-8.
    pub fn components(&self) -> Result<Vec<String>, UrlError> {
        self.segments()
            .map(|segment| decode_component(segment).map(Cow::into_owned))
            .collect()
    }

    /// The decoded final segment, such as `report.pdf` in `/docs/report.pdf`, or `None` for the empty path or a directory path.
    ///
    /// # Errors
    ///
    /// [`UrlError::Utf8`] when the segment does not decode to UTF-8.
    pub fn file_name(&self) -> Result<Option<String>, UrlError> {
        match self.encoded.rsplit('/').next() {
            Some(segment) if !segment.is_empty() => {
                Ok(Some(decode_component(segment)?.into_owned()))
            }
            _ => Ok(None),
        }
    }

    /// The path without its final segment: `/docs` for `/docs/report.pdf`, and `/docs` for `/docs/`. The empty path and `/` are their own parents.
    pub fn parent(&self) -> UrlPath {
        let Some(index) = self.encoded.rfind('/') else {
            return Self::new();
        };
        let parent = if index == 0 {
            "/"
        } else {
            &self.encoded[..index]
        };
        Self {
            encoded: parent.to_owned(),
        }
    }

    /// Resolve `reference` against this path as a URL would, following RFC 3986 section 5.2: the final segment of this path, a file name, is replaced.
    ///
    /// An absolute reference replaces this path, and an empty one leaves it unchanged. `.` and `..` segments are then removed; `..` cannot rise above the root of an absolute path, while a relative result keeps the `..` segments it cannot resolve.
    ///
    /// ```
    /// use base_web_url::UrlPath;
    ///
    /// let page = UrlPath::parse("/docs/guide/index.html")?;
    /// assert_eq!(page.resolve(&UrlPath::parse("../api/")?).as_str(), "/docs/api/");
    /// assert_eq!(page.resolve(&UrlPath::parse("style.css")?).as_str(), "/docs/guide/style.css");
    /// # Ok::<(), base_web_url::UrlError>(())
    /// ```
    pub fn resolve(&self, reference: &UrlPath) -> UrlPath {
        if reference.is_empty() {
            return self.clone();
        }
        if reference.is_absolute() {
            return reference.remove_dot_segments();
        }
        let directory = self
            .encoded
            .rfind('/')
            .map_or("", |index| &self.encoded[..=index]);
        Self {
            encoded: format!("{directory}{}", reference.encoded),
        }
        .remove_dot_segments()
    }

    /// Append `other` to this path as if this path were a directory, as [`std::path::Path::join`] does: `/api/users` joined with `42` is `/api/users/42`.
    ///
    /// An absolute `other` replaces this path, and an empty one leaves it unchanged. `.` and `..` segments are then removed, as for [`Self::resolve`].
    pub fn join(&self, other: &UrlPath) -> UrlPath {
        if other.is_empty() {
            return self.clone();
        }
        if other.is_absolute() || self.is_empty() {
            return other.remove_dot_segments();
        }
        let separator = if self.is_directory() { "" } else { "/" };
        Self {
            encoded: format!("{}{separator}{}", self.encoded, other.encoded),
        }
        .remove_dot_segments()
    }

    /// Remove `.` and `..` segments, including percent-encoded spellings such as `%2E%2E`, and collapse repeated `/` separators, without decoding other segments.
    ///
    /// This is lossy: `/a//b` and `/a/b` can name different resources. Use it when the application treats them alike, such as when mapping to files.
    pub fn simplify(&self) -> UrlPath {
        self.resolve_dot_segments(true)
    }

    /// Normalize the percent-encoding: decode escapes of unreserved characters, such as `%7E` to `~`, and use uppercase hexadecimal digits in the others, as RFC 3986 section 6.2.2 describes. The structure is unchanged.
    pub fn normalize(&self) -> UrlPath {
        let bytes = self.encoded.as_bytes();
        let mut encoded = String::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' {
                // Parsing checked that each `%` begins a complete escape.
                let (high, low) = (bytes[index + 1], bytes[index + 2]);
                let byte = hex_value(high) << 4 | hex_value(low);
                if is_unreserved(byte) {
                    encoded.push(char::from(byte));
                } else {
                    encoded.extend([
                        '%',
                        char::from(high.to_ascii_uppercase()),
                        char::from(low.to_ascii_uppercase()),
                    ]);
                }
                index += 3;
            } else {
                encoded.push(char::from(bytes[index]));
                index += 1;
            }
        }
        Self { encoded }
    }

    /// This absolute path as a reference relative to the directory of `base`, such as `../app.js` for `/assets/app.js` from `/docs/index.html`.
    ///
    /// Resolving the result against `base` with [`Self::resolve`] gives this path again. A path that is not absolute, or a `base` that is not, is returned unchanged.
    pub fn relative_to(&self, base: &UrlPath) -> UrlPath {
        if !self.is_absolute() || !base.is_absolute() {
            return self.clone();
        }
        let target: Vec<&str> = self.segments().collect();
        let mut directory: Vec<&str> = base.segments().collect();
        directory.pop();
        let mut common = target
            .iter()
            .zip(&directory)
            .take_while(|(target, directory)| target == directory)
            .count();
        // Keep at least the target's final segment: when the target is a prefix of the base's directory, as `/docs` from `/docs/index.html`, the reference must still name it. Both paths are absolute, so they share at least their empty first segment.
        if common == target.len() {
            common -= 1;
        }
        let mut segments: Vec<&str> = std::iter::repeat_n("..", directory.len() - common)
            .chain(target[common..].iter().copied())
            .collect();
        if segments == [""] {
            // The empty reference names the current document, so name the current directory explicitly.
            segments = vec![".", ""];
        } else if segments[0].contains(':') {
            // A `:` in the first segment would be read as a scheme.
            segments.insert(0, ".");
        }
        Self {
            encoded: segments.join("/"),
        }
    }

    /// The local filesystem path for this URL path beneath `root`, such as `/srv/public/docs/My Report.pdf` for `/docs/My%20Report.pdf`.
    ///
    /// Each segment is decoded once and must be exactly one file name. Empty and `.` segments are skipped, and `..` segments remove the previous file name. An absolute URL path is relative to `root`, not to the filesystem root.
    ///
    /// # Errors
    ///
    /// [`LocalPathError::EscapesRoot`] when `..` would leave `root`, and [`LocalPathError::Component`] when a segment does not decode to UTF-8 or contains NUL or a path separator, such as `%2F`.
    // SECURITY: Containment is lexical. Symbolic links beneath `root` are followed when the path is opened, and the filesystem can change between this check and that use, so the tree beneath `root` must be trusted not to contain links an attacker controls. Platform-specific names, such as Windows device names like `CON`, are not rejected.
    pub fn local_path(&self, root: impl AsRef<Path>) -> Result<PathBuf, LocalPathError> {
        let mut path = root.as_ref().to_path_buf();
        let mut depth = 0_usize;
        for segment in self.segments() {
            let name = decode_component(segment).map_err(|_| LocalPathError::Component)?;
            match &*name {
                "" | "." => {}
                ".." => {
                    depth = depth.checked_sub(1).ok_or(LocalPathError::EscapesRoot)?;
                    path.pop();
                }
                name if name.contains(RESERVED_FILE_NAME_CHARACTERS) => {
                    return Err(LocalPathError::Component);
                }
                name => {
                    path.push(name);
                    depth += 1;
                }
            }
        }
        Ok(path)
    }

    /// Remove literal `.` and `..` segments, as RFC 3986 section 5.2.4 describes.
    pub(crate) fn remove_dot_segments(&self) -> UrlPath {
        self.resolve_dot_segments(false)
    }

    /// Resolve dot segments, with `simplify` also treating percent-encoded dots as dot segments and collapsing empty segments.
    fn resolve_dot_segments(&self, simplify: bool) -> UrlPath {
        let absolute = self.is_absolute();
        let body = if absolute {
            &self.encoded[1..]
        } else {
            self.encoded.as_str()
        };
        if body.is_empty() {
            return self.clone();
        }
        let mut segments: Vec<&str> = Vec::new();
        // Whether the final segment named a directory, so the result needs a trailing `/`.
        let mut directory = false;
        for segment in body.split('/') {
            directory = true;
            match dot_segment(segment, simplify) {
                Some(DotSegment::Current) => {}
                Some(DotSegment::Parent) => match segments.last() {
                    Some(&last) if last != ".." => {
                        segments.pop();
                    }
                    // A relative path keeps the `..` segments it cannot resolve; an absolute one stops at its root.
                    _ if !absolute => segments.push(".."),
                    _ => {}
                },
                None if simplify && segment.is_empty() => {}
                None => {
                    segments.push(segment);
                    directory = segment.is_empty();
                }
            }
        }
        let mut encoded = String::from(if absolute { "/" } else { "" });
        encoded.push_str(&segments.join("/"));
        match segments.last() {
            Some(&last) if directory && !last.is_empty() => encoded.push('/'),
            None if directory && !absolute => encoded.push_str("./"),
            _ => {}
        }
        Self { encoded }
    }
}

/// Whether `segment` is a dot segment, optionally accepting `%2E` for each dot.
fn dot_segment(segment: &str, encoded: bool) -> Option<DotSegment> {
    let mut dots = 0;
    let mut rest = segment;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            rest = after;
        } else if encoded && rest.len() >= 3 && rest[..3].eq_ignore_ascii_case("%2e") {
            rest = &rest[3..];
        } else {
            return None;
        }
        dots += 1;
    }
    match dots {
        1 => Some(DotSegment::Current),
        2 => Some(DotSegment::Parent),
        _ => None,
    }
}

impl FromStr for UrlPath {
    type Err = UrlError;

    fn from_str(text: &str) -> Result<Self, UrlError> {
        Self::parse(text)
    }
}

impl AsRef<str> for UrlPath {
    fn as_ref(&self) -> &str {
        &self.encoded
    }
}

impl fmt::Display for UrlPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.encoded)
    }
}

impl fmt::Debug for UrlPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("UrlPath")
            .field(&self.encoded)
            .finish()
    }
}
