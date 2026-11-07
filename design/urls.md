# URL Syntax and Resolution

## Requirement

URLs with a scheme and URL references that may be relative must be distinct types: `Url` always has a scheme, `UrlReference` may not, and every `Url` converts into a `UrlReference`. Paths are a separate `UrlPath` type, and authorities a separate `Authority` type.

Parsing must follow the generic syntax of RFC 3986 exactly. A character outside the grammar of its component, a `%` not followed by two hexadecimal digits, an invalid scheme, user information, or host, a port that is not a number from 0 to 65535, and a path that cannot follow the other components are errors. Components are kept percent-encoded as written, except that the case-insensitive scheme is stored in lowercase; a query or fragment that is present but empty is kept distinct from an absent one, so writing a parsed reference reproduces its input apart from the scheme's case.

Reference resolution must follow RFC 3986 section 5.2 exactly, including its examples in section 5.4, and must keep empty path segments. Where the RFC's algorithm would produce a path that reads differently when written, a path beginning with `//` without an authority or a relative reference whose first segment contains `:`, the result gains a leading `/.` or `./`. Appending to a path as a directory is a separate operation, `join`, and collapsing repeated separators or treating percent-encoded dots as dot segments happens only through the explicit, lossy `simplify`.

Building a path from decoded values must encode each value as exactly one segment, so a `/` within a value never becomes a separator. Form content, including `application/x-www-form-urlencoded` queries, belongs to `base-web-content`.

The crate must have no dependencies, and must not depend on an HTTP or web framework crate.

## Rationale

The user chose separate types, mirroring `base-web-media-type`'s `MediaType` and `MediaRange`, so that a missing scheme is a type error rather than a runtime check. The user chose RFC 3986 exactly, both for parsing and for resolution, over the Ruby `protocol-url` gem's more permissive parser and its resolution, which collapses repeated slashes; the gem's directory-style joining, its `pop: false` option, is kept as `join` because the user relies on it. Keeping segments encoded and building paths from components preserves the gem's careful separation of URL syntax from decoded values. The user agreed that all form encoding, flat and nested, belongs to `base-web-content`, and the dependency rule follows the `base-web-*` family rule in `base-project`'s Package Organization guide.

## Scope

The `base-web-url` crate's `Url`, `UrlReference`, `UrlPath`, `Authority`, `UrlError`, `encode_component`, and `decode_component`, and their parsing, writing, comparison, resolution, and joining.

## Validation

The integration tests under `tests/` check accepted and rejected input for each component, round trips, every resolution example of RFC 3986 section 5.4, the adjustments for ambiguous results, and segment-preserving construction. Inspect `cargo tree -p base-web-url -e normal`, which must list no dependencies.
