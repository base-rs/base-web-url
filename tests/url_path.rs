// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_web_url::{LocalPathError, UrlError, UrlPath};
use std::path::{Path, PathBuf};

fn path(text: &str) -> UrlPath {
    UrlPath::parse(text).unwrap()
}

#[test]
fn keeps_encoded_segments_and_decodes_components() {
    let parsed = path("/a/b%2Fc/caf%C3%A9");
    assert_eq!(
        parsed.segments().collect::<Vec<_>>(),
        ["", "a", "b%2Fc", "caf%C3%A9"]
    );
    assert_eq!(parsed.components().unwrap(), ["", "a", "b/c", "caf\u{e9}"]);
    assert_eq!(path("").segments().count(), 0);
    assert_eq!(path("/").segments().collect::<Vec<_>>(), ["", ""]);
    assert_eq!(path("a/").segments().collect::<Vec<_>>(), ["a", ""]);
    assert_eq!(path("/%FF").components(), Err(UrlError::Utf8));
    assert_eq!(
        (parsed.to_string(), parsed.as_ref()),
        ("/a/b%2Fc/caf%C3%A9".to_owned(), "/a/b%2Fc/caf%C3%A9")
    );
    assert_eq!(format!("{parsed:?}"), "UrlPath(\"/a/b%2Fc/caf%C3%A9\")");
    assert_eq!("/x".parse::<UrlPath>(), Ok(path("/x")));
}

#[test]
fn builds_paths_from_segments_and_components() {
    assert_eq!(
        UrlPath::from_components(["", "a", "b/c", "d e"]).as_str(),
        "/a/b%2Fc/d%20e"
    );
    assert_eq!(UrlPath::from_components(["", ""]).as_str(), "/");
    assert_eq!(
        UrlPath::from_components(Vec::<String>::new()),
        UrlPath::new()
    );
    assert_eq!(
        UrlPath::from_segments(["", "a", "b%2Fc"]).unwrap().as_str(),
        "/a/b%2Fc"
    );
    assert_eq!(
        UrlPath::from_segments(["a/b"]),
        Err(UrlError::Syntax),
        "a segment cannot contain a separator"
    );
    assert_eq!(
        UrlPath::from_segments(["%zz"]),
        Err(UrlError::PercentEncoding)
    );
    for invalid in ["a b", "/a?b", "/a#b", "/\u{e9}"] {
        assert_eq!(
            UrlPath::parse(invalid),
            Err(UrlError::Syntax),
            "{invalid:?}"
        );
    }
    assert_eq!(UrlPath::parse("/%2"), Err(UrlError::PercentEncoding));
}

#[test]
fn inspects_paths() {
    assert!(path("/a").is_absolute() && !path("a").is_absolute());
    assert!(path("/a/").is_directory() && !path("/a").is_directory());
    assert!(UrlPath::new().is_empty() && UrlPath::default().is_empty());
    assert_eq!(
        path("/docs/My%20Report.pdf").file_name(),
        Ok(Some("My Report.pdf".to_owned()))
    );
    assert_eq!(
        path("report.pdf").file_name(),
        Ok(Some("report.pdf".to_owned()))
    );
    assert_eq!(path("/docs/").file_name(), Ok(None));
    assert_eq!(path("").file_name(), Ok(None));
    assert_eq!(path("/%FF").file_name(), Err(UrlError::Utf8));
    for (child, parent) in [
        ("/a/b", "/a"),
        ("a/b", "a"),
        ("/a/b/", "/a/b"),
        ("/a", "/"),
        ("a", ""),
        ("/", "/"),
        ("", ""),
        ("/a%2Fb/c", "/a%2Fb"),
    ] {
        assert_eq!(path(child).parent(), path(parent), "{child}");
    }
}

#[test]
fn resolves_references_as_a_url_would() {
    let page = path("/docs/guide/index.html");
    for (reference, expected) in [
        ("../api/", "/docs/api/"),
        ("style.css", "/docs/guide/style.css"),
        ("/x/../y", "/y"),
        ("", "/docs/guide/index.html"),
        (".", "/docs/guide/"),
        ("../../../../x", "/x"),
    ] {
        assert_eq!(
            page.resolve(&path(reference)),
            path(expected),
            "{reference:?}"
        );
    }
    // The base's file name is replaced, so a base without a trailing `/` loses its last segment.
    assert_eq!(
        path("/api/users").resolve(&path("groups")),
        path("/api/groups")
    );
    assert_eq!(
        path("/api/users/").resolve(&path("groups")),
        path("/api/users/groups")
    );
    // Relative paths keep the `..` segments they cannot resolve.
    assert_eq!(path("a/b").resolve(&path("../../c")), path("../c"));
    assert_eq!(path("file").resolve(&path("x")), path("x"));
    assert_eq!(path("a/b").resolve(&path("..")), path("./"));
    // Empty segments are kept, unlike `simplify`.
    assert_eq!(path("/a//b").resolve(&path("c")), path("/a//c"));
    assert_eq!(
        path("/a//b").resolve(&path("../c")),
        path("/a/c"),
        "`..` removes the empty segment, as RFC 3986 does"
    );
}

#[test]
fn joins_paths_as_directories() {
    for (base, other, expected) in [
        ("/api/users", "42", "/api/users/42"),
        ("/api/users/", "42", "/api/users/42"),
        ("/api", "../status", "/status"),
        ("/api", "/v2/x", "/v2/x"),
        ("/api", "", "/api"),
        ("", "a/./b", "a/b"),
        ("docs", "../../x", "../x"),
    ] {
        assert_eq!(
            path(base).join(&path(other)),
            path(expected),
            "{base} + {other}"
        );
    }
    assert_eq!(
        path("/files").join(&UrlPath::from_components(["My Report.pdf"])),
        path("/files/My%20Report.pdf")
    );
}

#[test]
fn simplifies_dot_segments_and_repeated_separators() {
    for (original, expected) in [
        ("a/b/../c/./d", "a/c/d"),
        ("/a/b/../../c", "/c"),
        ("/a/%2E%2e/b", "/b"),
        ("/a/.%2E/./%2e/b", "/b"),
        ("//a//b///c", "/a/b/c"),
        ("/a/b/", "/a/b/"),
        ("/a/b/.", "/a/b/"),
        ("../a/../../b", "../../b"),
        ("/../..", "/"),
        ("a/..", "./"),
        ("a%3Ab/c", "a%3Ab/c"),
        ("/...", "/..."),
        ("/%2e%2e%2e", "/%2e%2e%2e"),
        ("", ""),
        ("/", "/"),
    ] {
        assert_eq!(path(original).simplify(), path(expected), "{original}");
    }
    assert_eq!(path("/a/b").simplify().simplify(), path("/a/b"));
}

#[test]
fn normalizes_percent_encoding() {
    assert_eq!(path("/%41%7a%30%2D%2e%5F%7E").normalize(), path("/Az0-._~"));
    assert_eq!(path("/a%2fb%3fc%ff").normalize(), path("/a%2Fb%3Fc%FF"));
    assert_eq!(path("/%E2%9D%A4").normalize(), path("/%E2%9D%A4"));
    assert_eq!(
        path("//a/../welcome").normalize(),
        path("//a/../welcome"),
        "normalization keeps the structure"
    );
}

#[test]
fn expresses_paths_relative_to_a_base() {
    let cases = [
        (
            "/_components/app.js",
            "/foo/bar/",
            "../../_components/app.js",
        ),
        ("/docs/guide.html", "/docs/index.html", "guide.html"),
        ("/foo/bar/", "/", "foo/bar/"),
        ("/docs/", "/docs/api/reference.html", "../"),
        ("/docs/", "/docs/index.html", "./"),
        ("/", "/index.html", "./"),
        ("/docs", "/docs/index.html", "../docs"),
        ("/docs/this:that", "/docs/index.html", "./this:that"),
        ("/a/file.txt", "/x/y/z/", "../../../a/file.txt"),
        (
            "/projects/alpha/file.txt",
            "/projects/beta/",
            "../alpha/file.txt",
        ),
        ("/a%3Ab/d", "/a:b/c", "../a%3Ab/d"),
        ("/a/", "/a//x", "../"),
    ];
    for (target, base, expected) in cases {
        let relative = path(target).relative_to(&path(base));
        assert_eq!(relative, path(expected), "{target} from {base}");
        assert_eq!(
            path(base).resolve(&relative),
            path(target),
            "resolving {expected} against {base} gives {target} again"
        );
    }
    assert_eq!(
        path("a/b").relative_to(&path("/x")),
        path("a/b"),
        "a relative path is returned unchanged"
    );
    assert_eq!(
        path("/a/b").relative_to(&path("x")),
        path("/a/b"),
        "so is one relative to a relative base"
    );
}

#[test]
fn maps_paths_beneath_a_local_root() {
    let root = Path::new("/srv/public");
    let cases = [
        ("/documents/report.pdf", "documents/report.pdf"),
        ("documents/report.pdf", "documents/report.pdf"),
        ("/documents//report.pdf", "documents/report.pdf"),
        ("/files/My%20Document.txt", "files/My Document.txt"),
        ("/files/%E2%9D%A4.txt", "files/\u{2764}.txt"),
        ("/a/./b/../c", "a/c"),
        ("/a/%2E%2E/b", "b"),
        ("~/file.txt", "~/file.txt"),
        ("/files/name%21%40%23.txt", "files/name!@#.txt"),
        ("", ""),
        ("/", ""),
    ];
    for (url, local) in cases {
        assert_eq!(path(url).local_path(root), Ok(root.join(local)), "{url}");
    }
    for (url, error) in [
        ("/../etc/passwd", LocalPathError::EscapesRoot),
        ("/a/../../b", LocalPathError::EscapesRoot),
        ("/%2E%2E/x", LocalPathError::EscapesRoot),
        ("/folder/safe%2Fname/file.txt", LocalPathError::Component),
        ("/a%00b", LocalPathError::Component),
        ("/%FF", LocalPathError::Component),
    ] {
        assert_eq!(path(url).local_path(root), Err(error), "{url}");
    }
    assert_eq!(
        path("x").local_path(PathBuf::from("relative")),
        Ok(PathBuf::from("relative/x"))
    );
}

#[test]
fn builds_url_paths_from_local_paths() {
    assert_eq!(
        UrlPath::from_local_path("docs/My Report.pdf"),
        Ok(path("docs/My%20Report.pdf"))
    );
    assert_eq!(UrlPath::from_local_path("./a/../b%"), Ok(path("a/../b%25")));
    assert_eq!(UrlPath::from_local_path(""), Ok(UrlPath::new()));
    assert_eq!(
        UrlPath::from_local_path("/srv/public"),
        Err(LocalPathError::Absolute)
    );
    let local = Path::new("files/My Document.txt");
    let root = Path::new("/srv/public");
    assert_eq!(
        UrlPath::from_local_path(local).unwrap().local_path(root),
        Ok(root.join(local)),
        "the mapping round-trips"
    );
}

#[cfg(unix)]
#[test]
fn local_file_names_must_be_utf8() {
    use std::os::unix::ffi::OsStrExt;
    let name = std::ffi::OsStr::from_bytes(b"caf\xe9");
    assert_eq!(
        UrlPath::from_local_path(name),
        Err(LocalPathError::Component)
    );
}

#[test]
fn local_path_errors_describe_the_problem() {
    let messages: Vec<String> = [
        LocalPathError::EscapesRoot,
        LocalPathError::Component,
        LocalPathError::Absolute,
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        messages,
        [
            "URL path escapes the root directory",
            "URL path segment does not correspond to one file name",
            "local path is not relative"
        ]
    );
    let error: Box<dyn std::error::Error> = Box::new(LocalPathError::Component);
    assert!(error.source().is_none());
}
