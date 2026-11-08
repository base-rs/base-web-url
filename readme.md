# `base-web-url`

URLs, URL references, and URL paths for Rust, following RFC 3986, with percent-encoding, safe mapping to local files, and a streaming parser for form content and query strings.

A `Url` has a scheme, such as `https://example.com/docs/`; a `UrlReference` may also be relative, such as `../api?page=2`, and resolves against a page's URL following RFC 3986. A `UrlPath` keeps its percent-encoded segments, so a `/` inside a decoded value never becomes a separator, and maps to a file beneath a root directory without escaping it. `FormParser` reads `application/x-www-form-urlencoded` content and query strings strictly, as they arrive.

## Usage

```rust
use base_web_url::{Url, UrlReference};

let page = Url::parse("https://example.com/docs/guide/index.html").unwrap();
let link = page.resolve(UrlReference::parse("../My%20Report.pdf").unwrap());
assert_eq!(link.as_str(), "https://example.com/docs/My%20Report.pdf");

let api = Url::parse("https://api.example.com/v1").unwrap();
assert_eq!(api.join_component("a/b").unwrap().as_str(), "https://api.example.com/v1/a%2Fb");
```

See the guides:

- [Getting Started](context/getting-started.md): parsing, building, and resolving URLs and references.
- [Paths](context/paths.md): segments and components, joining and resolving, simplifying, and local files.
- [Forms and Query Strings](context/forms.md): reading form content and query strings, whole or as they arrive, and bracket paths.

## Releases

See [releases.md](releases.md).

## See Also

- [Protocol::URL](https://github.com/socketry/protocol-url), the Ruby gem this crate ports.
- [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986), the generic URI syntax.

## Contributing

Please open an issue or pull request on [GitHub](https://github.com/base-rs/base-web-url).

Install the [Bake launcher](https://github.com/base-rs/bake) and run `cargo bake test`. Use `cargo bake test:coverage --all-targets true` for the shared source-region coverage gate.

### Agent Context

Run `cargo bake agent:context:install` to install shared context and skills. Use `cargo bake agent:context:list` and `cargo bake agent:context:show` to discover relevant guides, follow repository guidance in `.agents/`, and apply skills under `.agents/skills/`.

Before planning or changing behavior, discover and read applicable `design/*.md`. Use `base-design-review` to assess plans and changes, and `base-design-author` when user requirements are introduced, changed, or removed. Review design compliance before opening or updating a pull request.
