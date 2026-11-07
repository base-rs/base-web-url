# Local Paths

## Requirement

`UrlPath::to_local_path` must map a URL path to a local path beneath a caller-supplied root, and must never return a path outside that root lexically. Each segment is percent-decoded exactly once and must correspond to exactly one file name: a segment that decodes to NUL or a path separator of the platform, such as `%2F`, is an error rather than being split or reinterpreted. On Unix, where file names are bytes, the decoded bytes become the file name exactly; on other platforms they must be UTF-8, and `\` and `:` are also rejected. Empty and `.` segments are skipped, and `..` segments, literal or percent-encoded, remove the previous name; a `..` that would leave the root is an error. An absolute URL path is relative to the root, not to the filesystem root.

`UrlPath::from_local_path` must encode each file name of a relative local path as exactly one segment, keep a trailing separator as a trailing `/`, and reject a path with a root or prefix and a file name containing NUL, or, on platforms other than Unix, one that is not Unicode. `UrlPath::from_local_path_under` must give the absolute URL path of a local path beneath a root, rejecting a path that does not begin with the root or whose `..` components would leave it. For a local path without `..` components, mapping it to a URL path and back beneath the same root must give the same local path.

Whenever `to_local_path` succeeds, the URL path after `normalize` and `simplify` must map to the same local path, so that access decisions made on that form see the file that is opened; the documentation must direct access decisions to the returned path or that form, never to the structure `resolve` or `parent` see, which keeps empty segments as levels. `from_local_path` and `from_local_path_under` must keep directory form for a local path ending with a separator, `.`, or `..`.

The guarantee is lexical containment only. The documentation must state that symbolic links are not resolved, that the filesystem can change between the check and its use, so the tree beneath the root must be trusted, and that one file can have several names on some platforms, so access decisions must not rest on the name alone.

## Rationale

The user considers mapping URL paths to local files, with correct escaping in both directions, an important part of the Ruby `protocol-url` gem's interface. These rules port its `Path#local_path` and `Encoding::System`, which enforce a one-to-one mapping between URL segments and file names so that encoded separators cannot smuggle extra components past the containment check. The user chose exact bytes on Unix, so that every file name can be linked and served, where the gem uses the filesystem's text encoding.

## Scope

`UrlPath::to_local_path`, `UrlPath::from_local_path`, `UrlPath::from_local_path_under`, and `LocalPathError`.

## Validation

The tests in `tests/url_path.rs` check mappings beneath a root, including encoded names, repeated separators, and dot segments; rejection of traversal above the root, encoded separators, and NUL; exact round trips of names that are not UTF-8 on Unix; trailing separators; that `from_local_path` and `from_local_path_under` round-trip through `to_local_path`, and, as a property over generated paths, that `to_local_path` agrees with the normalized and simplified path whenever it succeeds.
