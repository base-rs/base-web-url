# Local Paths

## Requirement

`UrlPath::local_path` must map a URL path to a local path beneath a caller-supplied root, and must never return a path outside that root lexically. Each segment is percent-decoded exactly once and must correspond to exactly one file name: a segment that decodes to bytes that are not UTF-8, or to text containing NUL or a path separator of the platform, such as `%2F`, is an error rather than being split or reinterpreted. Empty and `.` segments are skipped, and `..` segments, literal or percent-encoded, remove the previous name; a `..` that would leave the root is an error. An absolute URL path is relative to the root, not to the filesystem root.

`UrlPath::from_local_path` must encode each file name of a relative local path as exactly one segment, and must reject a path with a root or prefix and a file name that is not UTF-8. A path it produces must map back to the same local path beneath any root.

The guarantee is lexical containment only. The documentation must state that symbolic links are not resolved and that the filesystem can change between the check and its use, so the tree beneath the root must be trusted.

## Rationale

The user considers mapping URL paths to local files, with correct escaping in both directions, an important part of the Ruby `protocol-url` gem's interface. These rules port its `Path#local_path` and `Encoding::System`, which enforce a one-to-one mapping between URL segments and file names so that encoded separators cannot smuggle extra components past the containment check.

## Scope

`UrlPath::local_path`, `UrlPath::from_local_path`, and `LocalPathError`.

## Validation

The tests in `tests/url_path.rs` check mappings beneath a root, including encoded names, repeated separators, and dot segments, rejection of traversal above the root, encoded separators, NUL, and invalid UTF-8, and that `from_local_path` round-trips through `local_path`.
