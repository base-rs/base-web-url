# Releases

## Unreleased

- Add `Url` and `UrlReference`, parsed and written following RFC 3986 exactly, with `Authority` for user information, hosts including IP literals, and ports, and reference resolution that follows RFC 3986 section 5.2.
- Add `UrlPath`, which keeps percent-encoded segments distinct from decoded components, builds paths from values that can never navigate, resolves and joins paths, gives a path's file name, directory, parent, and ancestors, simplifies and normalizes paths, expresses them relative to a base, and maps them to and from local paths beneath a root, using exact bytes on Unix, without escaping it.
- Add `encode_component` and `decode_component` for percent-encoding application values.
- Add `normalize` on `Url`, `UrlReference`, `UrlPath`, and `Authority`, giving the normal form of RFC 3986 section 6, so that equivalent spellings, such as hosts in different cases, compare equal; and show passwords as `***` in `Debug` output.
