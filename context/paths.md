# Paths

This guide explains how `UrlPath` keeps encoded segments, how to combine paths safely, what a path's directory and parent are, and how to map URL paths to local files.

## Segments and components

A `UrlPath` is kept percent-encoded. A literal `/` separates segments, while `%2F` is data inside one segment, so the two never become confused:

```rust
use base_web_url::UrlPath;

let path = UrlPath::parse("/a/b%2Fc").unwrap();
assert_eq!(path.segments().collect::<Vec<_>>(), ["", "a", "b%2Fc"]);
assert_eq!(path.components().collect::<Result<Vec<_>, _>>().unwrap(), ["", "a", "b/c"]);
```

`segments` gives the encoded segments, for structural work, and `components` decodes each one as UTF-8. An absolute path's first segment is empty, as is a directory path's last.

## Building paths from values

Build a path from decoded application values with `from_components`, which encodes each value as exactly one segment, or append one value with `join_component`:

```rust
use base_web_url::{UrlError, UrlPath};

let path = UrlPath::from_components(["", "users", "a/b c"]).unwrap();
assert_eq!(path.as_str(), "/users/a%2Fb%20c");

let users = UrlPath::parse("/api/users").unwrap();
assert_eq!(users.join_component("../../etc").unwrap().as_str(), "/api/users/..%2F..%2Fetc");
assert_eq!(users.join_component(".."), Err(UrlError::Component));
```

A value is always data: it cannot add segments, navigate, or replace the path. A value of `.` or `..` is rejected, because a URL cannot express it as data; `%2E%2E` means the same as `..`. Never build paths by formatting values into a string, where a `/`, `?`, `#`, or `..` in a value would change the path's structure.

`from_segments` builds a path from segments that are already encoded URL syntax, so `..` there navigates.

## Files and directories

A path ending with `/` names a directory; any other path names a file within its directory. This decides how paths combine, and what their directory and parent are:

```rust
use base_web_url::UrlPath;

let report = UrlPath::parse("/docs/My%20Report.tar.gz").unwrap();
assert_eq!(report.file_name().unwrap().unwrap(), "My Report.tar.gz");
assert_eq!(report.file_stem().unwrap().unwrap(), "My Report.tar");
assert_eq!(report.extension().unwrap().unwrap(), "gz");
assert!(UrlPath::parse("/docs/").unwrap().file_name().is_none(), "a directory has no file name");
```

A decoded file name can contain `/` or NUL, from `%2F` or `%00`, so never append it to a local path yourself; map the whole path with `to_local_path`.

## Directories and parents

"Parent" can mean different things, so there are two operations, each with one meaning:

- `directory` is the directory containing the resource a path names: the path itself for a directory, and the path without its file name otherwise. It is the directory that relative references resolve within.
- `parent` is the directory one level above the resource, in directory form, like `std::path::Path::parent`. It is `None` at the root, and at the top of a relative path, where what lies above depends on the base the path is resolved against.

| Path | `directory()` | `parent()` |
| --- | --- | --- |
| `/docs/report.pdf` | `/docs/` | `/docs/` |
| `/docs/guide/` | `/docs/guide/` | `/docs/` |
| `/` | `/` | `None` |
| `/a/b/..` | `/a/` | `/` |
| `a/b` | `a/` | `a/` |
| `a` | `./` | `./` |
| `../` | `../` | `None` |
| `/a/b/%2E%2E` | `/a/` | `/` |

Both resolve `.` and `..` segments first, including percent-encoded ones such as `%2E%2E`, as a server that decodes them would, so neither is ever deeper than the path itself. An empty segment, as in `/a//b`, counts as a level, as it does when resolving `..`. Both return directory form, so resolving a reference without a leading `/` or `..` against them stays inside:

```rust
use base_web_url::UrlPath;

let report = UrlPath::parse("/docs/guide/report.pdf").unwrap();
let parent = report.parent().unwrap();
assert_eq!(parent.as_str(), "/docs/guide/");
assert_eq!(parent.resolve(&UrlPath::parse("index.html").unwrap()).as_str(), "/docs/guide/index.html");

let ancestors: Vec<String> = report.ancestors().map(|path| path.to_string()).collect();
assert_eq!(ancestors, ["/docs/guide/report.pdf", "/docs/guide/", "/docs/", "/"]);
```

These are URL semantics, not access control. For directories on disk, map the path with `to_local_path` and use `std::path::Path::parent`, which follows the filesystem's rules instead.

## Resolving and joining

There are two ways to combine paths, and the difference is whether the base's final segment is a file name:

- `resolve` treats the base as a URL does: when it names a file, that file name is replaced. This is the meaning of a link on a page.
- `join` treats the base as a directory, as `std::path::Path::join` does, and appends to it. The `url` crate's `Url::join`, and Ruby's `Protocol::URL::Path#join` by default, resolve instead.

```rust
use base_web_url::UrlPath;

let users = UrlPath::parse("/api/users").unwrap();
let groups = UrlPath::parse("groups").unwrap();
assert_eq!(users.resolve(&groups).as_str(), "/api/groups");
assert_eq!(users.join(&groups).as_str(), "/api/users/groups");

let page = UrlPath::parse("/docs/guide/index.html").unwrap();
assert_eq!(page.resolve(&UrlPath::parse("../api/").unwrap()).as_str(), "/docs/api/");
```

Both replace the base with an absolute path, leave it unchanged for an empty one, and remove `.` and `..` segments. `..` cannot rise above the root of an absolute path, and a relative result keeps the `..` segments it cannot resolve. Neither confines the result to the base: a path from an untrusted source can lead anywhere, so append untrusted values with `join_component`, and map to files with `to_local_path`, which does confine.

`relative_to` goes the other way, expressing an absolute path relative to a base, so resolving the result against that base gives the path back. `to_explicit` writes a relative path with a leading `./`, so that it cannot be mistaken for a name, as a module specifier in an import map can:

```rust
use base_web_url::UrlPath;

let script = UrlPath::parse("/assets/app.js").unwrap();
let page = UrlPath::parse("/assets/index.html").unwrap();
let link = script.relative_to(&page);
assert_eq!(link.as_str(), "app.js");
assert_eq!(link.to_explicit().as_str(), "./app.js");
assert_eq!(page.resolve(&link), script);
```

The result never begins with `/`, so it never becomes an absolute or host-relative reference, even for a path such as `/a//b`.

## Simplifying and normalizing

Paths compare by their exact encoded spelling. Two operations make equivalent spellings equal:

- `normalize` fixes the percent-encoding, decoding escapes of unreserved characters such as `%7E` and `%2E`, and using uppercase hexadecimal digits elsewhere.
- `simplify` removes `.` and `..` segments, including encoded spellings such as `%2E%2E`, and collapses repeated `/` separators. This is lossy, because `/a//b` and `/a/b` can name different resources, so use it only when the application treats them alike.

```rust
use base_web_url::UrlPath;

let path = UrlPath::parse("/a%7e//b/./c/%2E%2E/d").unwrap();
assert_eq!(path.normalize().as_str(), "/a~//b/./c/../d");
assert_eq!(path.normalize().simplify().as_str(), "/a~/b/d");
```

## Local files

`to_local_path` maps a URL path to a file beneath a root directory, such as for a static file server. Each segment is decoded exactly once and must become exactly one file name:

```rust
use base_web_url::{LocalPathError, UrlPath};
use std::path::Path;

let root = Path::new("/srv/public");
let path = UrlPath::parse("/files/My%20Document.txt").unwrap();
assert_eq!(path.to_local_path(root).unwrap(), root.join("files/My Document.txt"));

let traversal = UrlPath::parse("/../../etc/passwd").unwrap();
assert_eq!(traversal.to_local_path(root), Err(LocalPathError::EscapesRoot));

let smuggled = UrlPath::parse("/folder/a%2F..%2F..%2Fsecret").unwrap();
assert_eq!(smuggled.to_local_path(root), Err(LocalPathError::Component));
```

An absolute URL path is relative to the root, not to the filesystem root. Empty and `.` segments are skipped, and `..` removes the previous name but can never leave the root; percent-encoded dots count as dots here, as servers commonly decode them. A segment that decodes to a path separator or NUL is rejected rather than reinterpreted. On Unix, file names are bytes, so any decoded bytes are used exactly; elsewhere they must be UTF-8.

The other direction encodes file names, so links to files are always escaped correctly. `from_local_path_under` gives the absolute URL path of a file beneath a root, and `from_local_path` encodes a relative local path:

```rust
use base_web_url::UrlPath;

let url = UrlPath::from_local_path_under("/srv/public", "/srv/public/files/My Document #2.txt").unwrap();
assert_eq!(url.as_str(), "/files/My%20Document%20%232.txt");

let link = UrlPath::from_local_path("files/").unwrap();
assert_eq!(link.as_str(), "files/");
```

### Access decisions

The URL operations and the filesystem can see different structure. `resolve` and `parent` follow RFC 3986, which counts an empty segment as a level, while the filesystem treats `//` like `/`, and `resolve` treats `%2E%2E` as data, while `to_local_path` decodes it to `..`. So decide access on the path `to_local_path` returns, or on the URL path after `normalize` and `simplify`, which maps to the same file whenever `to_local_path` succeeds, and never on the path as `resolve` or `parent` see it:

```rust
use base_web_url::UrlPath;
use std::path::Path;

let root = Path::new("/srv");
let request = UrlPath::parse("/public//../private/x").unwrap();
assert_eq!(UrlPath::parse("/").unwrap().resolve(&request).as_str(), "/public/private/x");
assert_eq!(request.normalize().simplify().as_str(), "/private/x");
assert_eq!(request.to_local_path(root).unwrap(), root.join("private/x"));
```

The containment check is lexical. It does not resolve symbolic links, and the filesystem can change between the check and opening the file, so the tree beneath the root must not contain links that an untrusted party controls. One file can also have several names, such as by letter case on macOS and Windows, so do not base access decisions on the name alone.
