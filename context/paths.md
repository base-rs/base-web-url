# Paths

This guide explains how `UrlPath` keeps encoded segments, how to join and simplify paths, and how to map URL paths to local files safely.

## Segments and components

A `UrlPath` is kept percent-encoded. A literal `/` separates segments, while `%2F` is data inside one segment, so the two never become confused:

```rust
use base_web_url::UrlPath;

let path = UrlPath::parse("/a/b%2Fc").unwrap();
assert_eq!(path.segments().collect::<Vec<_>>(), ["", "a", "b%2Fc"]);
assert_eq!(path.components().unwrap(), ["", "a", "b/c"]);
```

`segments` gives the encoded segments, for structural work, and `components` decodes them as UTF-8. An absolute path's first segment is empty, as is a directory path's last.

Build a path from decoded application values with `from_components`, which encodes each value as exactly one segment, or from encoded segments with `from_segments`:

```rust
use base_web_url::UrlPath;

let path = UrlPath::from_components(["", "users", "a/b c"]);
assert_eq!(path.as_str(), "/users/a%2Fb%20c");

assert_eq!(UrlPath::from_segments(["", "users", "a%2Fb"]).unwrap().as_str(), "/users/a%2Fb");
assert!(UrlPath::from_segments(["a/b"]).is_err(), "an encoded segment cannot contain `/`");
```

Never build paths by formatting decoded values into a string: a value containing `/`, `?`, or `#` would change the path's structure.

## Inspecting paths

```rust
use base_web_url::UrlPath;

let path = UrlPath::parse("/releases/My%20Archive.tar.gz").unwrap();
assert_eq!(path.file_name().unwrap().as_deref(), Some("My Archive.tar.gz"));
assert_eq!(path.parent().as_str(), "/releases");
assert!(path.is_absolute() && !path.is_directory());
```

## Resolving and joining

There are two ways to combine paths, and the difference is whether the base's final segment is a file name:

- `resolve` treats the base as a URL does: its final segment, unless it ends with `/`, is a file name and is replaced.
- `join` treats the base as a directory, as `std::path::Path::join` does, and appends to it.

```rust
use base_web_url::UrlPath;

let users = UrlPath::parse("/api/users").unwrap();
let groups = UrlPath::parse("groups").unwrap();
assert_eq!(users.resolve(&groups).as_str(), "/api/groups");
assert_eq!(users.join(&groups).as_str(), "/api/users/groups");

let page = UrlPath::parse("/docs/guide/index.html").unwrap();
assert_eq!(page.resolve(&UrlPath::parse("../api/").unwrap()).as_str(), "/docs/api/");
```

Both replace the base with an absolute path, leave it unchanged for an empty one, and remove `.` and `..` segments. `..` cannot rise above the root of an absolute path; a relative result keeps the `..` segments it cannot resolve.

`relative_to` goes the other way, expressing an absolute path relative to a page, so resolving the result against that page gives the path back:

```rust
use base_web_url::UrlPath;

let script = UrlPath::parse("/assets/app.js").unwrap();
let page = UrlPath::parse("/docs/guide/index.html").unwrap();
let link = script.relative_to(&page);
assert_eq!(link.as_str(), "../../assets/app.js");
assert_eq!(page.resolve(&link), script);
```

## Simplifying and normalizing

Paths compare by their exact encoded spelling. Two operations make equivalent spellings equal:

- `normalize` fixes the percent-encoding, decoding escapes of unreserved characters such as `%7E` and using uppercase hexadecimal digits elsewhere. It never changes the structure.
- `simplify` removes `.` and `..` segments, including encoded spellings such as `%2E%2E`, and collapses repeated `/` separators. This is lossy, because `/a//b` and `/a/b` can name different resources, so use it only when the application treats them alike.

```rust
use base_web_url::UrlPath;

let path = UrlPath::parse("/a%7e//b/./c/%2E%2E/d").unwrap();
assert_eq!(path.normalize().as_str(), "/a~//b/./c/../d");
assert_eq!(path.normalize().simplify().as_str(), "/a~/b/d");
```

## Local files

`local_path` maps a URL path to a file beneath a root directory, such as for a static file server. Each segment is decoded exactly once and must become exactly one file name:

```rust
use base_web_url::{LocalPathError, UrlPath};
use std::path::Path;

let root = Path::new("/srv/public");
let path = UrlPath::parse("/files/My%20Document.txt").unwrap();
assert_eq!(path.local_path(root).unwrap(), root.join("files/My Document.txt"));

let traversal = UrlPath::parse("/../../etc/passwd").unwrap();
assert_eq!(traversal.local_path(root), Err(LocalPathError::EscapesRoot));

let smuggled = UrlPath::parse("/folder/a%2F..%2F..%2Fsecret").unwrap();
assert_eq!(smuggled.local_path(root), Err(LocalPathError::Component));
```

An absolute URL path is relative to the root, not to the filesystem root. Empty and `.` segments are skipped, and `..` removes the previous name but can never leave the root. A segment that decodes to a path separator, NUL, or bytes that are not UTF-8 is rejected rather than reinterpreted.

`from_local_path` goes the other way, encoding each file name of a relative local path as one segment, so links to files are always escaped correctly:

```rust
use base_web_url::UrlPath;

let link = UrlPath::from_local_path("files/My Document #2.txt").unwrap();
assert_eq!(link.as_str(), "files/My%20Document%20%232.txt");
```

The containment check is lexical. It does not resolve symbolic links, and the filesystem can change between the check and opening the file, so the tree beneath the root must not contain links that an untrusted party controls.
