// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! URLs, URL references, and URL paths, following RFC 3986, with percent-encoding, local file paths, and form content.
//!
//! A [`Url`] has a scheme, such as `https://example.com/docs/`. A [`UrlReference`] may also be relative, such as `../api?page=2`, as in an HTML link, and [`Url::resolve`] resolves it against a page's URL. A [`UrlPath`] keeps its percent-encoded segments, so a `/` inside a decoded value never becomes a separator, and maps safely to a local file path beneath a root.
//!
//! ```
//! use base_web_url::{Url, UrlPath, UrlReference};
//!
//! let page = Url::parse("https://example.com/docs/guide/index.html")?;
//! let link = page.resolve(UrlReference::parse("../My%20Report.pdf")?);
//! assert_eq!(link.as_str(), "https://example.com/docs/My%20Report.pdf");
//!
//! let file = link.path().to_local_path("/srv/public").unwrap();
//! assert_eq!(file, std::path::Path::new("/srv/public/docs/My Report.pdf"));
//! # Ok::<(), base_web_url::UrlError>(())
//! ```
//!
//! The [Getting Started](https://github.com/base-rs/base-web-url/blob/main/context/getting-started.md) guide covers parsing, building, and resolving URLs, and the [Paths](https://github.com/base-rs/base-web-url/blob/main/context/paths.md) guide covers path segments, encoding, joining, and local files. [`FormParser`] and [`form_pairs`] read `application/x-www-form-urlencoded` content and query strings, as the [Forms and Query Strings](https://github.com/base-rs/base-web-url/blob/main/context/forms.md) guide describes.

mod authority;
mod form_error;
mod form_event;
mod form_limits;
mod form_name;
mod form_pairs;
mod form_parser;
mod form_path;
mod local_path_error;
mod percent_encoding;
mod syntax;
mod url;
mod url_error;
mod url_path;
mod url_reference;

pub use authority::Authority;
pub use form_error::{FormError, FormErrorKind};
pub use form_event::{FormEvent, FormParsed};
pub use form_limits::FormLimits;
pub use form_name::FormName;
pub use form_pairs::{FormPair, FormPairs, form_pairs};
pub use form_parser::FormParser;
pub use form_path::{FormPath, FormPathSegment, FormPathSegments};
pub use local_path_error::LocalPathError;
pub use percent_encoding::{decode_component, encode_component};
pub use url::Url;
pub use url_error::UrlError;
pub use url_path::UrlPath;
pub use url_reference::UrlReference;

#[cfg(doctest)]
#[doc = include_str!("../readme.md")]
struct Readme;

#[cfg(doctest)]
#[doc = include_str!("../context/getting-started.md")]
struct GettingStartedGuide;

#[cfg(doctest)]
#[doc = include_str!("../context/paths.md")]
struct PathsGuide;

#[cfg(doctest)]
#[doc = include_str!("../context/forms.md")]
struct FormsGuide;
