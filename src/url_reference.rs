// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::syntax::{self, is_query_byte};
use crate::{Authority, UrlError, UrlPath};
use std::fmt;
use std::str::FromStr;

/// A URL reference: a URL, such as `https://example.com/docs/`, or a relative reference, such as `../api?page=2#results`, following the `URI-reference` grammar of RFC 3986.
///
/// Components are kept percent-encoded, as written, except that the case-insensitive scheme is stored in lowercase. A query or fragment that is present but empty, as in `/search?`, is kept distinct from an absent one. Equality compares the components exactly.
///
/// ```
/// use base_web_url::UrlReference;
///
/// let reference = UrlReference::parse("../api/users?page=2#results")?;
/// assert!(reference.is_relative());
/// assert_eq!(reference.path().as_str(), "../api/users");
/// assert_eq!((reference.query(), reference.fragment()), (Some("page=2"), Some("results")));
/// # Ok::<(), base_web_url::UrlError>(())
/// ```
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UrlReference {
    scheme: Option<String>,
    authority: Option<Authority>,
    path: UrlPath,
    query: Option<String>,
    fragment: Option<String>,
}

impl UrlReference {
    /// Parse a URL or relative reference, following RFC 3986 exactly: characters outside the grammar, such as spaces, `|`, or unencoded non-ASCII text, are errors, as are malformed percent-encodings.
    ///
    /// # Errors
    ///
    /// The [`UrlError`] variant names the component that is invalid.
    pub fn parse(text: &str) -> Result<Self, UrlError> {
        let parts = syntax::split(text);
        let scheme = match parts.scheme {
            Some(scheme) if syntax::is_scheme(scheme) => Some(scheme.to_ascii_lowercase()),
            Some(_) => return Err(UrlError::Scheme),
            None => None,
        };
        let reference = Self {
            scheme,
            authority: parts.authority.map(Authority::parse).transpose()?,
            path: UrlPath::parse(parts.path)?,
            query: parts.query.map(validate_query).transpose()?,
            fragment: parts.fragment.map(validate_query).transpose()?,
        };
        reference.check_path()?;
        Ok(reference)
    }

    /// The same reference with the scheme `scheme`, or none.
    ///
    /// # Errors
    ///
    /// [`UrlError::Scheme`] for an invalid scheme, and [`UrlError::Path`] when removing the scheme leaves a `:` in the first path segment.
    pub fn with_scheme(mut self, scheme: Option<&str>) -> Result<Self, UrlError> {
        self.scheme = match scheme {
            Some(scheme) if syntax::is_scheme(scheme) => Some(scheme.to_ascii_lowercase()),
            Some(_) => return Err(UrlError::Scheme),
            None => None,
        };
        self.check_path()?;
        Ok(self)
    }

    /// The same reference with the authority `authority`, or none.
    ///
    /// # Errors
    ///
    /// [`UrlError::Path`] when the path cannot follow the new authority, or no authority.
    pub fn with_authority(mut self, authority: Option<Authority>) -> Result<Self, UrlError> {
        self.authority = authority;
        self.check_path()?;
        Ok(self)
    }

    /// The same reference with the path `path`.
    ///
    /// # Errors
    ///
    /// [`UrlError::Path`] when a path after an authority is not empty and does not begin with `/`, a path without an authority begins with `//`, or the first segment of a reference without a scheme contains `:`.
    pub fn with_path(mut self, path: UrlPath) -> Result<Self, UrlError> {
        self.path = path;
        self.check_path()?;
        Ok(self)
    }

    /// The same reference with the percent-encoded query `query`, without its `?`, or none.
    ///
    /// # Errors
    ///
    /// [`UrlError::Syntax`] for a character outside the query grammar, such as a space or `#`, and [`UrlError::PercentEncoding`] for a malformed escape. Encode application values with [`crate::encode_component`].
    pub fn with_query(mut self, query: Option<&str>) -> Result<Self, UrlError> {
        self.query = query.map(validate_query).transpose()?;
        Ok(self)
    }

    /// The same reference with the percent-encoded fragment `fragment`, without its `#`, or none.
    ///
    /// # Errors
    ///
    /// As for [`Self::with_query`].
    pub fn with_fragment(mut self, fragment: Option<&str>) -> Result<Self, UrlError> {
        self.fragment = fragment.map(validate_query).transpose()?;
        Ok(self)
    }

    /// The scheme in lowercase, such as `https`, or `None` for a relative reference.
    pub fn scheme(&self) -> Option<&str> {
        self.scheme.as_deref()
    }

    /// The authority, such as `example.com:8080`.
    pub fn authority(&self) -> Option<&Authority> {
        self.authority.as_ref()
    }

    /// The path, which may be empty.
    pub fn path(&self) -> &UrlPath {
        &self.path
    }

    /// The percent-encoded query, without its `?`.
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// The percent-encoded fragment, without its `#`. Decode it with [`crate::decode_component`].
    pub fn fragment(&self) -> Option<&str> {
        self.fragment.as_deref()
    }

    /// Whether this is a relative reference, without a scheme.
    pub fn is_relative(&self) -> bool {
        self.scheme.is_none()
    }

    /// Resolve `reference` against this reference as its base, following RFC 3986 section 5.2.
    ///
    /// A reference with a scheme replaces the base; one with an authority keeps only the base's scheme; an empty path keeps the base's path and, without a query of its own, its query; and any other path is resolved with [`UrlPath::resolve`]. The fragment always comes from `reference`. The base may itself be relative, in which case the result is relative too.
    ///
    /// ```
    /// use base_web_url::UrlReference;
    ///
    /// let base = UrlReference::parse("/docs/guide/index.html?version=2")?;
    /// assert_eq!(base.resolve(&UrlReference::parse("../api/#users")?).to_string(), "/docs/api/#users");
    /// assert_eq!(base.resolve(&UrlReference::parse("#top")?).to_string(), "/docs/guide/index.html?version=2#top");
    /// # Ok::<(), base_web_url::UrlError>(())
    /// ```
    #[must_use]
    pub fn resolve(&self, reference: impl AsRef<UrlReference>) -> UrlReference {
        let reference = reference.as_ref();
        let scheme = reference.scheme.clone().or_else(|| self.scheme.clone());
        // With a scheme, dot segments are removed exactly as RFC 3986 section 5.2.4 does, even from a path without a leading `/`; a relative result keeps the `..` segments it cannot resolve.
        let exact = scheme.is_some();
        let remove_dot_segments = |path: &UrlPath| {
            if exact {
                path.remove_dot_segments_rfc()
            } else {
                path.remove_dot_segments()
            }
        };
        let fragment = reference.fragment.clone();
        if reference.scheme.is_some() || reference.authority.is_some() {
            return Self::from_parts(
                scheme,
                reference.authority.clone(),
                remove_dot_segments(&reference.path),
                reference.query.clone(),
                fragment,
            );
        }
        let (path, query) = if reference.path.is_empty() {
            (
                self.path.clone(),
                reference.query.clone().or_else(|| self.query.clone()),
            )
        } else if reference.path.is_absolute() {
            (
                remove_dot_segments(&reference.path),
                reference.query.clone(),
            )
        } else if self.authority.is_some() && self.path.is_empty() {
            // A base with an authority and an empty path merges as if its path were `/`.
            (
                remove_dot_segments(&root().merge(&reference.path)),
                reference.query.clone(),
            )
        } else {
            (
                remove_dot_segments(&self.path.merge(&reference.path)),
                reference.query.clone(),
            )
        };
        Self::from_parts(scheme, self.authority.clone(), path, query, fragment)
    }

    /// This reference relative to `base`, such as `../api/?page=2` for `https://example.com/api/?page=2` from `https://example.com/docs/index.html`, keeping its query and fragment; see [`UrlPath::relative_to`].
    ///
    /// Resolving the result against `base` gives this reference again, with dot segments removed from its path. A reference with a different scheme or authority from `base`, or a path that is not absolute, is returned unchanged.
    #[must_use]
    pub fn relative_to(&self, base: impl AsRef<UrlReference>) -> UrlReference {
        let base = base.as_ref();
        let comparable = match (&self.scheme, &self.authority) {
            (Some(_), _) => self.scheme == base.scheme && self.authority == base.authority,
            (None, Some(_)) => self.authority == base.authority,
            (None, None) => true,
        };
        if !comparable || !self.path.is_absolute() || !base.path.is_absolute() {
            return self.clone();
        }
        Self::from_parts(
            None,
            None,
            self.path.relative_to(&base.path),
            self.query.clone(),
            self.fragment.clone(),
        )
    }

    /// Append `path` to this reference's path as if it were a directory, as for [`UrlPath::join`], removing the query and fragment, which belonged to the original resource. A reference with an authority and an empty path, such as `https://example.com`, joins as if its path were `/`.
    #[must_use]
    pub fn join(&self, path: &UrlPath) -> UrlReference {
        let base = if self.authority.is_some() && self.path.is_empty() {
            root()
        } else {
            self.path.clone()
        };
        Self::from_parts(
            self.scheme.clone(),
            self.authority.clone(),
            base.join(path),
            None,
            None,
        )
    }

    /// This reference written explicitly as a relative reference when it is a relative path, such as `./guide.html` for `guide.html`; see [`UrlPath::to_explicit`]. References with a scheme, an authority, or an absolute or empty path are unchanged.
    #[must_use]
    pub fn to_explicit(&self) -> UrlReference {
        if self.scheme.is_some() || self.authority.is_some() {
            return self.clone();
        }
        Self {
            path: self.path.to_explicit(),
            ..self.clone()
        }
    }

    /// A reference from resolved components, adjusting a path that would otherwise be read differently: `//` without an authority would begin an authority, so it becomes `/.//`, and a `:` in the first segment of a relative reference would end a scheme, so the path gains a leading `./`.
    pub(crate) fn from_parts(
        scheme: Option<String>,
        authority: Option<Authority>,
        path: UrlPath,
        query: Option<String>,
        fragment: Option<String>,
    ) -> Self {
        let mut reference = Self {
            scheme,
            authority,
            path,
            query,
            fragment,
        };
        if reference.check_path().is_err() {
            // Paths after an authority are always empty or absolute when resolved, so only the two ambiguous forms reach here.
            let prefix = if reference.path.is_absolute() {
                "/."
            } else {
                "./"
            };
            reference.path = UrlPath::parse(&format!("{prefix}{}", reference.path))
                .expect("a dot segment keeps the path valid");
        }
        reference
    }

    /// A relative reference with only an authority and a path, not yet checked; callers check it when they add the scheme.
    pub(crate) fn from_authority_and_path(authority: Option<Authority>, path: UrlPath) -> Self {
        Self {
            authority,
            path,
            ..Self::default()
        }
    }

    fn check_path(&self) -> Result<(), UrlError> {
        let path = self.path.as_str();
        let valid = if self.authority.is_some() {
            path.is_empty() || path.starts_with('/')
        } else {
            !path.starts_with("//")
                && (self.scheme.is_some()
                    || !path
                        .split('/')
                        .next()
                        .is_some_and(|segment| segment.contains(':')))
        };
        if valid { Ok(()) } else { Err(UrlError::Path) }
    }
}

fn validate_query(text: &str) -> Result<String, UrlError> {
    syntax::validate(text, is_query_byte, UrlError::Syntax)?;
    Ok(text.to_owned())
}

impl From<UrlPath> for UrlReference {
    /// A relative reference to `path`, adjusted as for resolution when the path alone would be read as a scheme or an authority, such as `./a:b` for `a:b`.
    fn from(path: UrlPath) -> Self {
        Self::from_parts(None, None, path, None, None)
    }
}

impl FromStr for UrlReference {
    type Err = UrlError;

    fn from_str(text: &str) -> Result<Self, UrlError> {
        Self::parse(text)
    }
}

impl fmt::Display for UrlReference {
    /// Writes the reference as RFC 3986 section 5.3 recomposes it.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scheme) = &self.scheme {
            write!(formatter, "{scheme}:")?;
        }
        if let Some(authority) = &self.authority {
            write!(formatter, "//{authority}")?;
        }
        formatter.write_str(self.path.as_str())?;
        if let Some(query) = &self.query {
            write!(formatter, "?{query}")?;
        }
        if let Some(fragment) = &self.fragment {
            write!(formatter, "#{fragment}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for UrlReference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("UrlReference")
            .field(&format_args!("{self}"))
            .finish()
    }
}

/// The root path, `/`.
fn root() -> UrlPath {
    UrlPath::parse("/").expect("`/` is a valid path")
}
