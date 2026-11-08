# Releases

## Unreleased

- Add `FormParser`, a sans-I/O parser for `application/x-www-form-urlencoded` content and query strings, following the WHATWG URL Standard with strict decoding: it reports each complete name before its value, streams values in decoded pieces, and limits size, pair count, and name and value lengths.
- Add `form_pairs`, for form content and query strings held in memory, and `FormName::path` and `FormPath::parse`, which read bracketed names such as `user[emails][]` as segments, with a depth limit.

## v0.1.0

- Add `Url` and `UrlReference`, parsed and written following RFC 3986 exactly, with `Authority` for user information, hosts including IP literals, and ports, and reference resolution that follows RFC 3986 section 5.2.
- Add `UrlPath`, which keeps percent-encoded segments distinct from decoded components, builds paths from values that can never navigate, resolves and joins paths, gives a path's file name, directory, parent, and ancestors, simplifies and normalizes paths, expresses them relative to a base, and maps them to and from local paths beneath a root, using exact bytes on Unix, without escaping it.
- Add `encode_component` and `decode_component` for percent-encoding application values.
- Add `normalize` on `Url`, `UrlReference`, `UrlPath`, and `Authority`, giving the normal form of RFC 3986 section 6, so that equivalent spellings, such as hosts in different cases, compare equal; and show passwords as `***` in `Debug` output.
