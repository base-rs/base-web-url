# Getting Started

This guide explains how to parse, build, and resolve URLs with `base-web-url`.

## Core concepts

- A `Url` has a scheme, such as `https://example.com/docs/` or `mailto:ada@example.com`.
- A `UrlReference` may also be relative, such as `../api?page=2#results`, as in an HTML link. Every `Url` is a `UrlReference`.
- A `UrlPath` is the path of either, kept percent-encoded so segment boundaries are never lost. The [Paths](paths.md) guide covers it.
- An `Authority` is the `user@host:port` part of a URL.

Parsing follows RFC 3986 exactly: spaces, `|`, unencoded non-ASCII text, and malformed `%` escapes are errors, never silently passed through. Components are kept percent-encoded as written, except that the scheme is stored in lowercase.

## Parsing

```rust
use base_web_url::{Url, UrlReference};

let url = Url::parse("https://user@example.com:8080/docs/a%20b?page=2#results").unwrap();
assert_eq!(url.scheme(), "https");
let authority = url.authority().unwrap();
assert_eq!((authority.userinfo(), authority.host(), authority.port()), (Some("user"), "example.com", Some(8080)));
assert_eq!(url.path().as_str(), "/docs/a%20b");
assert_eq!((url.query(), url.fragment()), (Some("page=2"), Some("results")));

let link = UrlReference::parse("../api/users?page=2").unwrap();
assert!(link.is_relative());
assert!(Url::parse("../api/users").is_err(), "a URL needs a scheme");
```

A query or fragment that is present but empty, as in `/search?`, is kept distinct from an absent one, so every parsed reference is written back exactly as it was parsed, apart from the scheme's case.

## Building

Build URLs from components, and replace components with the `with_*` methods, which validate what they are given:

```rust
use base_web_url::{Authority, Url, UrlPath, encode_component};

let url = Url::new("https", Some(Authority::parse("example.com").unwrap()), UrlPath::parse("/search").unwrap()).unwrap();
let query = format!("q={}", encode_component("fish & chips"));
let url = url.with_query(Some(&query)).unwrap();
assert_eq!(url.as_str(), "https://example.com/search?q=fish%20%26%20chips");
```

Components passed to `with_query` and `with_fragment` must already be percent-encoded; encode application values with `encode_component`, which encodes everything but unreserved characters, and decode them with `decode_component`. Form content, such as `application/x-www-form-urlencoded` queries with `+` for spaces, belongs to `base-web-content`.

## Resolving links

`Url::resolve` resolves a reference against a page's URL as a browser does, following RFC 3986 section 5.2:

```rust
use base_web_url::{Url, UrlReference};

let page = Url::parse("https://example.com/docs/guide/index.html?version=2").unwrap();
let resolve = |link: &str| page.resolve(UrlReference::parse(link).unwrap()).to_string();

assert_eq!(resolve("../api/"), "https://example.com/docs/api/");
assert_eq!(resolve("style.css"), "https://example.com/docs/guide/style.css");
assert_eq!(resolve("/about"), "https://example.com/about");
assert_eq!(resolve("#top"), "https://example.com/docs/guide/index.html?version=2#top");
assert_eq!(resolve("//cdn.example.com/app.js"), "https://cdn.example.com/app.js");
```

The page's final segment, `index.html`, is a file name and is replaced. A base whose path ends with `/` names a directory, so references resolve inside it. `UrlReference::resolve` does the same for a base that is itself relative, such as resolving links between pages of a site without knowing its host.

## Joining paths

To add path segments to a URL, such as building API endpoints, use `Url::join`, which treats the URL's path as a directory whether or not it ends with `/`:

```rust
use base_web_url::{Url, UrlPath};

let api = Url::parse("https://api.example.com/v1").unwrap();
let user = api.join(&UrlPath::from_components(["users", "Ada Lovelace"]));
assert_eq!(user.as_str(), "https://api.example.com/v1/users/Ada%20Lovelace");
```

`join` removes the query and fragment, which belonged to the original resource. Use `resolve` when you mean what a link would mean, and `join` when you are appending to a path.
