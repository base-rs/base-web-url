// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::url_reference::Redacted;
use crate::{Authority, UrlError, UrlPath, UrlReference};
use std::fmt;
use std::str::FromStr;

/// A URL with a scheme, such as `https://example.com/docs/?page=2`, the `URI` of RFC 3986.
///
/// A URL is a [`UrlReference`] that is not relative, so it converts into one and can be used wherever a reference is accepted through `AsRef<UrlReference>`. Converting a relative reference into a URL fails.
///
/// ```
/// use base_web_url::{Url, UrlReference};
///
/// let page = Url::parse("https://example.com/docs/guide/index.html")?;
/// assert_eq!(page.resolve(UrlReference::parse("../api/")?).as_str(), "https://example.com/docs/api/");
///
/// let api = Url::parse("https://api.example.com/v1")?;
/// assert_eq!(api.join_component("users")?.join_component("Ada Lovelace")?.as_str(), "https://api.example.com/v1/users/Ada%20Lovelace");
/// # Ok::<(), base_web_url::UrlError>(())
/// ```
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Url {
    reference: UrlReference,
    /// The written form, cached so [`Self::as_str`] can borrow it.
    text: String,
}

impl Url {
    /// Parse a URL, following RFC 3986 exactly, as for [`UrlReference::parse`].
    ///
    /// # Errors
    ///
    /// As for [`UrlReference::parse`], and [`UrlError::Relative`] when there is no scheme.
    pub fn parse(text: &str) -> Result<Self, UrlError> {
        UrlReference::parse(text)?.try_into()
    }

    /// A URL from a scheme, an optional authority, and a path, such as `https`, `example.com`, and `/docs/`.
    ///
    /// # Errors
    ///
    /// [`UrlError::Scheme`] for an invalid scheme, and [`UrlError::Path`] when the path cannot follow the authority, or no authority.
    pub fn new(
        scheme: &str,
        authority: Option<Authority>,
        path: UrlPath,
    ) -> Result<Self, UrlError> {
        // Setting the authority and path together lets a path such as `//x` follow an authority; `with_scheme` then checks them.
        UrlReference::from_authority_and_path(authority, path)
            .with_scheme(Some(scheme))?
            .try_into()
    }

    /// The scheme in lowercase, such as `https`.
    pub fn scheme(&self) -> &str {
        self.reference.scheme().expect("a URL has a scheme")
    }

    /// The authority, such as `example.com:8080`.
    pub fn authority(&self) -> Option<&Authority> {
        self.reference.authority()
    }

    /// The path, which may be empty.
    pub fn path(&self) -> &UrlPath {
        self.reference.path()
    }

    /// The percent-encoded query, without its `?`.
    pub fn query(&self) -> Option<&str> {
        self.reference.query()
    }

    /// The percent-encoded fragment, without its `#`.
    pub fn fragment(&self) -> Option<&str> {
        self.reference.fragment()
    }

    /// The URL as text, recomposed from its components, so the scheme is in lowercase.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// This URL as a reference.
    pub fn as_reference(&self) -> &UrlReference {
        &self.reference
    }

    /// The same URL with the scheme `scheme`, such as `https`.
    ///
    /// # Errors
    ///
    /// [`UrlError::Scheme`] for an invalid scheme.
    pub fn with_scheme(self, scheme: &str) -> Result<Self, UrlError> {
        Ok(Self::from_reference(
            self.reference.with_scheme(Some(scheme))?,
        ))
    }

    /// The same URL with the authority `authority`, or none, as for [`UrlReference::with_authority`].
    ///
    /// # Errors
    ///
    /// As for [`UrlReference::with_authority`].
    pub fn with_authority(self, authority: Option<Authority>) -> Result<Self, UrlError> {
        Ok(Self::from_reference(
            self.reference.with_authority(authority)?,
        ))
    }

    /// The same URL with the path `path`, as for [`UrlReference::with_path`].
    ///
    /// # Errors
    ///
    /// As for [`UrlReference::with_path`].
    pub fn with_path(self, path: UrlPath) -> Result<Self, UrlError> {
        Ok(Self::from_reference(self.reference.with_path(path)?))
    }

    /// The same URL with the percent-encoded query `query`, or none, as for [`UrlReference::with_query`].
    ///
    /// # Errors
    ///
    /// As for [`UrlReference::with_query`].
    pub fn with_query(self, query: Option<&str>) -> Result<Self, UrlError> {
        Ok(Self::from_reference(self.reference.with_query(query)?))
    }

    /// The same URL with the percent-encoded fragment `fragment`, or none, as for [`UrlReference::with_fragment`].
    ///
    /// # Errors
    ///
    /// As for [`UrlReference::with_fragment`].
    pub fn with_fragment(self, fragment: Option<&str>) -> Result<Self, UrlError> {
        Ok(Self::from_reference(
            self.reference.with_fragment(fragment)?,
        ))
    }

    /// Resolve `reference` against this URL, following RFC 3986 section 5.2, as for [`UrlReference::resolve`].
    ///
    /// This is the generic URI resolution of RFC 3986, not the WHATWG URL Standard that browsers follow: browsers also accept and correct input that RFC 3986 rejects, and treat a few cases, such as `http:g`, differently.
    #[must_use]
    pub fn resolve(&self, reference: impl AsRef<UrlReference>) -> Url {
        Self::from_reference(self.reference.resolve(reference))
    }

    /// The same URL in the normal form of RFC 3986 section 6, as for [`UrlReference::normalize`].
    #[must_use]
    pub fn normalize(&self) -> Url {
        Self::from_reference(self.reference.normalize())
    }

    /// This URL relative to `base`, keeping its query and fragment, as for [`UrlReference::relative_to`].
    #[must_use]
    pub fn relative_to(&self, base: impl AsRef<UrlReference>) -> UrlReference {
        self.reference.relative_to(base)
    }

    /// Append `path` to this URL's path as if it were a directory, as for [`UrlReference::join`], removing the query and fragment, which belonged to the original resource. A URL with an authority and an empty path, such as `https://example.com`, joins as if its path were `/`.
    ///
    /// `..` in `path` can leave this URL's path; use [`Self::join_component`] to append an untrusted value.
    #[must_use]
    pub fn join(&self, path: &UrlPath) -> Url {
        Self::from_reference(self.reference.join(path))
    }

    /// Append one decoded application value as a single path segment, as for [`UrlPath::join_component`], removing the query and fragment.
    ///
    /// # Errors
    ///
    /// [`UrlError::Component`] for an empty value, `.`, or `..`.
    pub fn join_component(&self, component: &str) -> Result<Url, UrlError> {
        if component.is_empty() {
            return Err(UrlError::Component);
        }
        Ok(self.join(&UrlPath::from_components([component])?))
    }

    /// A URL from a reference that is known to have a scheme.
    fn from_reference(reference: UrlReference) -> Self {
        let text = reference.to_string();
        Self { reference, text }
    }
}

impl AsRef<UrlReference> for Url {
    fn as_ref(&self) -> &UrlReference {
        &self.reference
    }
}

impl AsRef<UrlReference> for UrlReference {
    fn as_ref(&self) -> &UrlReference {
        self
    }
}

impl From<Url> for UrlReference {
    fn from(url: Url) -> Self {
        url.reference
    }
}

impl TryFrom<UrlReference> for Url {
    type Error = UrlError;

    /// Fails with [`UrlError::Relative`] when the reference has no scheme.
    fn try_from(reference: UrlReference) -> Result<Self, UrlError> {
        if reference.is_relative() {
            return Err(UrlError::Relative);
        }
        Ok(Self::from_reference(reference))
    }
}

impl FromStr for Url {
    type Err = UrlError;

    fn from_str(text: &str) -> Result<Self, UrlError> {
        Self::parse(text)
    }
}

impl fmt::Display for Url {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

impl fmt::Debug for Url {
    /// Shows the URL as written, such as `Url(https://user:***@example.com/)`, with any password replaced by `***`, so that debugging output and logs do not reveal it.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("Url")
            .field(&format_args!("{}", Redacted(&self.reference)))
            .finish()
    }
}

impl PartialEq<UrlReference> for Url {
    fn eq(&self, other: &UrlReference) -> bool {
        self.reference == *other
    }
}

impl PartialEq<Url> for UrlReference {
    fn eq(&self, other: &Url) -> bool {
        *self == other.reference
    }
}
