# Releases

## Unreleased

- Add `Url` and `UrlReference`, parsed and written following RFC 3986 exactly, with `Authority` for user information, hosts including IP literals, and ports, and reference resolution that follows RFC 3986 section 5.2.
- Add `UrlPath`, which keeps percent-encoded segments distinct from decoded components, resolves and joins paths, simplifies and normalizes them, expresses them relative to a base, and maps them to and from local paths beneath a root without escaping it.
- Add `encode_component` and `decode_component` for percent-encoding application values.
