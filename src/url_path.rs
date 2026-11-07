// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::percent_encoding::{decode_bytes, encode_bytes};
use crate::syntax::{self, hex_value, is_path_byte, is_segment_byte, is_unreserved};
use crate::{LocalPathError, UrlError, decode_component};
use std::borrow::{Borrow, Cow};
use std::ffi::{OsStr, OsString};
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
/// let path = UrlPath::from_components(["", "docs", "a/b c"])?;
/// assert_eq!(path.as_str(), "/docs/a%2Fb%20c");
/// assert_eq!(path.segments().collect::<Vec<_>>(), ["", "docs", "a%2Fb%20c"]);
/// assert_eq!(path.components().collect::<Result<Vec<_>, _>>()?, ["", "docs", "a/b c"]);
/// # Ok::<(), base_web_url::UrlError>(())
/// ```
///
/// A path ending with `/` names a directory, and any other names a file within its directory; this decides how [`Self::resolve`] treats a base, and what [`Self::directory`] and [`Self::parent`] return. `.` and `..` segments navigate when a path is resolved, joined, or asked for its parent; a percent-encoded dot, such as `%2E%2E`, is data, except to [`Self::simplify`] and [`Self::to_local_path`].
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
    /// Segments are URL syntax, so `..` navigates; use [`Self::from_components`] for application values.
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

    /// A path from decoded application values, each percent-encoded as exactly one segment, so a `/` within a value stays data. A first value that is empty makes the path absolute, and a last one that is empty gives it a trailing `/`.
    ///
    /// # Errors
    ///
    /// [`UrlError::Component`] for a value of `.` or `..`, which a URL cannot express as data: `%2E%2E` means the same as `..`, so the value would navigate instead of naming something.
    pub fn from_components<C: AsRef<str>>(
        components: impl IntoIterator<Item = C>,
    ) -> Result<Self, UrlError> {
        let mut segments = Vec::new();
        for component in components {
            let component = component.as_ref();
            if matches!(component, "." | "..") {
                return Err(UrlError::Component);
            }
            segments.push(encode_bytes(component.as_bytes()));
        }
        Ok(Self {
            encoded: segments.join("/"),
        })
    }

    /// A relative URL path from a relative local path, encoding each file name as one segment, such as `docs/My%20Report.pdf` for `docs/My Report.pdf`. A trailing separator becomes a trailing `/`.
    ///
    /// `.` components are dropped, and `..` components are kept, so the result can link from one file to another. On Unix, file names are bytes and are encoded exactly; elsewhere they must be Unicode.
    ///
    /// # Errors
    ///
    /// [`LocalPathError::Absolute`] for a path with a root or prefix, such as `/srv` or `C:\srv`, and [`LocalPathError::Component`] for a file name containing NUL, or one that is not Unicode on a platform other than Unix.
    pub fn from_local_path(path: impl AsRef<Path>) -> Result<Self, LocalPathError> {
        let path = path.as_ref();
        let mut segments = Vec::new();
        for component in path.components() {
            match component {
                Component::Normal(name) => segments.push(url_segment(name)?),
                Component::CurDir => {}
                Component::ParentDir => segments.push("..".to_owned()),
                Component::RootDir | Component::Prefix(_) => return Err(LocalPathError::Absolute),
            }
        }
        if names_directory(path) {
            if segments.is_empty() {
                segments.push(".".to_owned());
            }
            segments.push(String::new());
        }
        Ok(Self {
            encoded: segments.join("/"),
        })
    }

    /// The absolute URL path of `path` beneath `root`, the inverse of [`Self::to_local_path`]: `/docs/My%20Report.pdf` for `/srv/public/docs/My Report.pdf` beneath `/srv/public`. The root itself is `/`, and a trailing separator becomes a trailing `/`.
    ///
    /// # Errors
    ///
    /// [`LocalPathError::EscapesRoot`] when `path` does not begin with `root`, or its `..` components would leave it, and [`LocalPathError::Component`] as for [`Self::from_local_path`]. Both paths are compared as written, without consulting the filesystem.
    pub fn from_local_path_under(
        root: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> Result<Self, LocalPathError> {
        let path = path.as_ref();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| LocalPathError::EscapesRoot)?;
        let mut segments = vec![String::new()];
        for component in relative.components() {
            match component {
                Component::Normal(name) => segments.push(url_segment(name)?),
                Component::ParentDir if segments.len() == 1 => {
                    return Err(LocalPathError::EscapesRoot);
                }
                Component::ParentDir => {
                    segments.pop();
                }
                // The remainder of a path after its root is relative, so it has no root or prefix.
                Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            }
        }
        if segments.len() == 1 || names_directory(path) {
            segments.push(String::new());
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

    /// The segments, decoded as UTF-8, as for [`Self::segments`]. A segment without escapes is borrowed.
    ///
    /// Each item is [`UrlError::Utf8`] when its segment does not decode to UTF-8.
    pub fn components(&self) -> impl Iterator<Item = Result<Cow<'_, str>, UrlError>> {
        self.segments().map(decode_component)
    }

    /// The decoded final segment, such as `My Report.pdf` in `/docs/My%20Report.pdf`.
    ///
    /// `None` for the empty path, a directory path, and a final `.` or `..`, which name directories. Unlike [`std::path::Path::file_name`], a trailing `/` is significant: `/docs/` names the directory itself, which has no file name. The item is [`UrlError::Utf8`] when the segment does not decode to UTF-8.
    pub fn file_name(&self) -> Option<Result<Cow<'_, str>, UrlError>> {
        let segment = self
            .encoded
            .rsplit('/')
            .next()
            .filter(|segment| !segment.is_empty() && dot_segment(segment, true).is_none())?;
        Some(decode_component(segment))
    }

    /// The file name without its final extension, such as `archive.tar` for `archive.tar.gz`, as [`std::path::Path::file_stem`] does: a leading dot, as in `.profile`, does not begin an extension.
    pub fn file_stem(&self) -> Option<Result<Cow<'_, str>, UrlError>> {
        Some(
            self.file_name()?
                .map(|name| map_cow(name, |name| split_file_name(name).0)),
        )
    }

    /// The final extension of the file name, without its dot, such as `gz` for `archive.tar.gz`, or `None` when the file name has none.
    pub fn extension(&self) -> Option<Result<Cow<'_, str>, UrlError>> {
        match self.file_name()? {
            Ok(name) => {
                split_file_name(&name).1?;
                Some(Ok(map_cow(name, |name| {
                    split_file_name(name)
                        .1
                        .expect("the file name has an extension")
                })))
            }
            Err(error) => Some(Err(error)),
        }
    }

    /// The directory containing the resource this path names: the path itself when it names a directory, and otherwise the path without its file name, such as `/docs/` for `/docs/report.pdf`.
    ///
    /// This is the directory that a relative reference resolved against this path is relative to. Dot segments are resolved first, and a relative path without a directory gives `./`.
    #[must_use]
    pub fn directory(&self) -> UrlPath {
        let path = self.remove_dot_segments();
        if path.is_directory() {
            return path;
        }
        match path.encoded.rfind('/') {
            Some(index) => Self {
                encoded: path.encoded[..=index].to_owned(),
            },
            None => Self {
                encoded: "./".to_owned(),
            },
        }
    }

    /// The directory one level above the resource this path names, in directory form: `/docs/` for both `/docs/report.pdf` and `/docs/guide/`.
    ///
    /// Dot segments are resolved first, so the parent is never deeper than the path: the parent of `/a/b/..` is `/`. An empty segment counts as a level, as it does when resolving `..`. `None` at the root `/`, and for a relative path whose remaining segments are all `..`, such as `./` or `../`, since what lies above depends on the base it is resolved against.
    ///
    /// Use [`std::path::Path::parent`] on a local path from [`Self::to_local_path`] to work with directories on disk instead.
    pub fn parent(&self) -> Option<UrlPath> {
        let path = self.remove_dot_segments();
        let named = path.encoded.strip_suffix('/').unwrap_or(&path.encoded);
        let last = named
            .rsplit('/')
            .next()
            .expect("splitting yields a segment");
        if (path.is_absolute() && named.is_empty())
            || (!path.is_absolute() && matches!(last, "" | "." | ".."))
        {
            return None;
        }
        Some(match named.rfind('/') {
            Some(index) => Self {
                encoded: named[..=index].to_owned(),
            },
            None => Self {
                encoded: "./".to_owned(),
            },
        })
    }

    /// This path followed by each of its parents in turn, as for [`Self::parent`], ending at the root or at the top of a relative path.
    pub fn ancestors(&self) -> impl Iterator<Item = UrlPath> {
        std::iter::successors(Some(self.clone()), UrlPath::parent)
    }

    /// Resolve `reference` against this path as a URL would, following RFC 3986 section 5.2: when this path names a file, that file name is replaced.
    ///
    /// An absolute reference replaces this path, and an empty one leaves it unchanged. `.` and `..` segments are then removed; `..` cannot rise above the root of an absolute path, while a relative result keeps the `..` segments it cannot resolve. The result can be anywhere: use [`Self::join_component`] to append an untrusted value beneath this path.
    ///
    /// ```
    /// use base_web_url::UrlPath;
    ///
    /// let page = UrlPath::parse("/docs/guide/index.html")?;
    /// assert_eq!(page.resolve(&UrlPath::parse("../api/")?).as_str(), "/docs/api/");
    /// assert_eq!(page.resolve(&UrlPath::parse("style.css")?).as_str(), "/docs/guide/style.css");
    /// # Ok::<(), base_web_url::UrlError>(())
    /// ```
    #[must_use]
    pub fn resolve(&self, reference: &UrlPath) -> UrlPath {
        if reference.is_empty() {
            return self.clone();
        }
        if reference.is_absolute() {
            return reference.remove_dot_segments();
        }
        self.merge(reference).remove_dot_segments()
    }

    /// Append `other` to this path as if this path were a directory, as [`std::path::Path::join`] does: `/api/users` joined with `42` is `/api/users/42`. Note that the `url` crate's `Url::join` resolves instead, as [`Self::resolve`] does here.
    ///
    /// An absolute `other` replaces this path, and an empty one returns this path unchanged; otherwise dot segments are removed from the joined path, so `..` in `other` can leave this path. Use [`Self::join_component`] to append an untrusted value.
    #[must_use]
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

    /// Append one decoded application value as a single segment beneath this path, as if this path were a directory: `/api/users` with `Ada Lovelace` is `/api/users/Ada%20Lovelace`.
    ///
    /// The value is always data: it cannot add segments, navigate, or replace the path.
    ///
    /// # Errors
    ///
    /// [`UrlError::Component`] for an empty value, `.`, or `..`.
    pub fn join_component(&self, component: &str) -> Result<UrlPath, UrlError> {
        if component.is_empty() {
            return Err(UrlError::Component);
        }
        Ok(self.join(&Self::from_components([component])?))
    }

    /// Remove `.` and `..` segments, including percent-encoded spellings such as `%2E%2E`, and collapse repeated `/` separators, without decoding other segments.
    ///
    /// This is lossy: `/a//b` and `/a/b` can name different resources. Use it when the application treats them alike, such as when comparing paths that map to files.
    #[must_use]
    pub fn simplify(&self) -> UrlPath {
        self.resolve_dot_segments(true)
    }

    /// Normalize the percent-encoding: decode escapes of unreserved characters, such as `%7E` to `~` and `%2E` to `.`, and use uppercase hexadecimal digits in the others, as RFC 3986 section 6.2.2 describes. Segments are unchanged, though a decoded dot can make a segment a dot segment.
    #[must_use]
    pub fn normalize(&self) -> UrlPath {
        let bytes = self.encoded.as_bytes();
        let mut encoded = String::with_capacity(bytes.len());
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' {
                // Every constructor checks that each `%` begins a complete escape.
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

    /// This absolute path as a reference relative to `base`, such as `../app.js` for `/assets/app.js` from `/docs/index.html`.
    ///
    /// Dot segments are removed from both paths first. Resolving the result against `base` with its dot segments removed, using [`Self::resolve`], gives this path with its dot segments removed; resolving it against a `base` whose final segment is `.` or `..` can differ, because RFC 3986 merges a reference after such a segment rather than after the directory it names. The result never begins with `/`, and begins with `./` when it would otherwise be empty, start with an empty segment, or have a `:` in its first segment. A path that is not absolute, or a `base` that is not, is returned unchanged.
    #[must_use]
    pub fn relative_to(&self, base: &UrlPath) -> UrlPath {
        if !self.is_absolute() || !base.is_absolute() {
            return self.clone();
        }
        let (target, base) = (self.remove_dot_segments(), base.remove_dot_segments());
        let target: Vec<&str> = target.segments().collect();
        let mut directory: Vec<&str> = base.segments().collect();
        directory.pop();
        let mut common = target
            .iter()
            .zip(&directory)
            .take_while(|(target, directory)| target == directory)
            .count();
        // Keep at least the target's final segment, so the reference still names it when the target is a prefix of the base's directory, as `/docs` is from `/docs/index.html`. Both paths are absolute, so they share their empty first segment.
        if common == target.len() {
            common -= 1;
        }
        let mut segments: Vec<&str> = std::iter::repeat_n("..", directory.len() - common)
            .chain(target[common..].iter().copied())
            .collect();
        // An empty first segment would make the reference absolute, or empty, which names the current document; a `:` would end a scheme.
        if segments[0].is_empty() || segments[0].contains(':') {
            segments.insert(0, ".");
        }
        Self {
            encoded: segments.join("/"),
        }
    }

    /// This path written explicitly as a relative path when it is one: `./guide.html` for `guide.html`, so that it cannot be mistaken for a name, as a module specifier in an import map can. Absolute and empty paths, and those already beginning with `./` or `../`, are unchanged.
    #[must_use]
    pub fn to_explicit(&self) -> UrlPath {
        let first = self.segments().next().unwrap_or("");
        if self.is_empty() || self.is_absolute() || matches!(first, "." | "..") {
            return self.clone();
        }
        Self {
            encoded: format!("./{}", self.encoded),
        }
    }

    /// The local filesystem path for this URL path beneath `root`, such as `/srv/public/docs/My Report.pdf` for `/docs/My%20Report.pdf`.
    ///
    /// Each segment is decoded once and must correspond to exactly one file name. Empty and `.` segments are skipped, and `..` segments, including percent-encoded ones, remove the previous file name but cannot leave `root`. An absolute URL path is relative to `root`, not to the filesystem root. On Unix, decoded bytes become the file name exactly; elsewhere they must be UTF-8.
    ///
    /// # Errors
    ///
    /// [`LocalPathError::EscapesRoot`] when `..` would leave `root`, and [`LocalPathError::Component`] when a segment decodes to NUL or a path separator, such as `%2F`, or, on platforms other than Unix, to something that is not UTF-8 or contains `\` or `:`.
    ///
    /// # Security
    ///
    /// Containment is lexical. Symbolic links beneath `root` are followed when the path is opened, and the filesystem can change between this call and that use, so the tree beneath `root` must not contain links that an untrusted party controls. One file can also have several names, so do not base access decisions on the name alone: on Windows, trailing dots and spaces, letter case, short `8.3` names, and device names such as `CON`; on macOS, letter case and Unicode normalization.
    // SECURITY: Containment is lexical: links and races beneath `root`, and platform name aliasing, are the caller's to manage, as the documentation above states.
    pub fn to_local_path(&self, root: impl AsRef<Path>) -> Result<PathBuf, LocalPathError> {
        let mut path = root.as_ref().to_path_buf();
        let mut depth = 0_usize;
        for segment in self.segments() {
            let name = decode_bytes(segment).expect("every constructor checks the escapes");
            match &*name {
                b"" | b"." => {}
                b".." => {
                    depth = depth.checked_sub(1).ok_or(LocalPathError::EscapesRoot)?;
                    path.pop();
                }
                name => {
                    path.push(local_file_name(name)?);
                    depth += 1;
                }
            }
        }
        Ok(path)
    }

    /// The directory part of this path, up to and including its final `/`, followed by `reference`, before dot segments are removed, as RFC 3986 section 5.2.3 merges paths.
    pub(crate) fn merge(&self, reference: &UrlPath) -> UrlPath {
        let directory = self
            .encoded
            .rfind('/')
            .map_or("", |index| &self.encoded[..=index]);
        Self {
            encoded: format!("{directory}{}", reference.encoded),
        }
    }

    /// Remove literal `.` and `..` segments. For an absolute path this is the algorithm of RFC 3986 section 5.2.4; a relative path keeps the `..` segments it cannot resolve.
    pub(crate) fn remove_dot_segments(&self) -> UrlPath {
        self.resolve_dot_segments(false)
    }

    /// Remove dot segments exactly as the algorithm of RFC 3986 section 5.2.4 does, for paths of references with a scheme, including those without a leading `/`.
    pub(crate) fn remove_dot_segments_rfc(&self) -> UrlPath {
        let mut input = self.encoded.as_str();
        let mut output = String::with_capacity(input.len());
        while !input.is_empty() {
            if let Some(rest) = input
                .strip_prefix("../")
                .or_else(|| input.strip_prefix("./"))
            {
                input = rest;
            } else if input.starts_with("/./") || input == "/." {
                input = if input == "/." { "/" } else { &input[2..] };
            } else if input.starts_with("/../") || input == "/.." {
                input = if input == "/.." { "/" } else { &input[3..] };
                output.truncate(output.rfind('/').unwrap_or(0));
            } else if input == "." || input == ".." {
                input = "";
            } else {
                let end = input[1..].find('/').map_or(input.len(), |index| index + 1);
                output.push_str(&input[..end]);
                input = &input[end..];
            }
        }
        Self { encoded: output }
    }

    /// Resolve dot segments, with `simplify` also treating percent-encoded dots as dot segments and collapsing empty segments.
    fn resolve_dot_segments(&self, simplify: bool) -> UrlPath {
        if self.encoded.is_empty() {
            return Self::new();
        }
        let absolute = self.is_absolute();
        let body = if absolute {
            &self.encoded[1..]
        } else {
            self.encoded.as_str()
        };
        let parts: Vec<&str> = body.split('/').collect();
        let last = parts.len() - 1;
        let mut segments: Vec<&str> = Vec::with_capacity(parts.len());
        for (index, &segment) in parts.iter().enumerate() {
            match dot_segment(segment, simplify) {
                Some(dot) => {
                    if dot == DotSegment::Parent {
                        match segments.last() {
                            Some(&previous) if previous != ".." => {
                                segments.pop();
                            }
                            // A relative path keeps the `..` segments it cannot resolve; an absolute one stops at its root.
                            _ if !absolute => segments.push(".."),
                            _ => {}
                        }
                    }
                    // A final dot segment names a directory.
                    if index == last {
                        segments.push("");
                    }
                }
                None if simplify && segment.is_empty() && index != last => {}
                None => segments.push(segment),
            }
        }
        let mut encoded = String::from(if absolute { "/" } else { "" });
        // A relative result beginning with an empty segment would read as absolute, or as the current document when it is only that, so it begins with `./`.
        if !absolute && segments[0].is_empty() {
            encoded.push_str("./");
        }
        encoded.push_str(&segments.join("/"));
        Self { encoded }
    }
}

/// Whether `segment` is a dot segment, optionally accepting `%2E` for each dot.
fn dot_segment(segment: &str, encoded: bool) -> Option<DotSegment> {
    let mut dots = 0;
    let mut rest = segment.as_bytes();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix(b".") {
            rest = after;
        } else if encoded && rest.len() >= 3 && rest[..3].eq_ignore_ascii_case(b"%2e") {
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

/// A file name's stem and final extension, as [`std::path::Path::file_stem`] and [`std::path::Path::extension`] split them.
fn split_file_name(name: &str) -> (&str, Option<&str>) {
    match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem, Some(extension)),
        _ => (name, None),
    }
}

/// Apply `part` to the text of `name`, borrowing when `name` is borrowed.
fn map_cow<'a>(name: Cow<'a, str>, part: impl Fn(&str) -> &str) -> Cow<'a, str> {
    match name {
        Cow::Borrowed(name) => Cow::Borrowed(part(name)),
        Cow::Owned(name) => Cow::Owned(part(&name).to_owned()),
    }
}

/// Whether a local path ends with a separator, naming a directory.
fn names_directory(path: &Path) -> bool {
    path.as_os_str()
        .as_encoded_bytes()
        .last()
        .is_some_and(|&byte| std::path::is_separator(char::from(byte)))
}

/// One file name, encoded as one URL segment.
#[cfg(unix)]
fn url_segment(name: &OsStr) -> Result<String, LocalPathError> {
    use std::os::unix::ffi::OsStrExt;
    let bytes = name.as_bytes();
    if bytes.contains(&0) {
        return Err(LocalPathError::Component);
    }
    Ok(encode_bytes(bytes))
}

/// One file name, encoded as one URL segment.
#[cfg(not(unix))]
fn url_segment(name: &OsStr) -> Result<String, LocalPathError> {
    let name = name.to_str().ok_or(LocalPathError::Component)?;
    if name.contains('\0') {
        return Err(LocalPathError::Component);
    }
    Ok(encode_bytes(name.as_bytes()))
}

/// One decoded URL segment as one file name.
#[cfg(unix)]
fn local_file_name(bytes: &[u8]) -> Result<OsString, LocalPathError> {
    use std::os::unix::ffi::OsStringExt;
    if bytes.contains(&0) || bytes.contains(&b'/') {
        return Err(LocalPathError::Component);
    }
    Ok(OsString::from_vec(bytes.to_vec()))
}

/// One decoded URL segment as one file name. On Windows, `:` would name a drive or an alternate data stream.
#[cfg(not(unix))]
fn local_file_name(bytes: &[u8]) -> Result<OsString, LocalPathError> {
    let name = std::str::from_utf8(bytes).map_err(|_| LocalPathError::Component)?;
    if name.contains(['\0', '/', '\\', ':']) {
        return Err(LocalPathError::Component);
    }
    Ok(OsString::from(name))
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

impl Borrow<str> for UrlPath {
    fn borrow(&self) -> &str {
        &self.encoded
    }
}

impl PartialEq<str> for UrlPath {
    fn eq(&self, other: &str) -> bool {
        self.encoded == other
    }
}

impl PartialEq<&str> for UrlPath {
    fn eq(&self, other: &&str) -> bool {
        self.encoded == *other
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
