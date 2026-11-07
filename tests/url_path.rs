// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_web_url::{LocalPathError, UrlError, UrlPath};
use std::path::{Path, PathBuf};

fn path(text: &str) -> UrlPath {
    UrlPath::parse(text).unwrap()
}

fn components(path: &UrlPath) -> Vec<String> {
    path.components()
        .map(|component| component.unwrap().into_owned())
        .collect()
}

#[test]
fn keeps_encoded_segments_and_decodes_components() {
    let parsed = path("/a/b%2Fc/caf%C3%A9");
    assert_eq!(
        parsed.segments().collect::<Vec<_>>(),
        ["", "a", "b%2Fc", "caf%C3%A9"]
    );
    assert_eq!(components(&parsed), ["", "a", "b/c", "caf\u{e9}"]);
    assert!(
        matches!(
            parsed.components().nth(1),
            Some(Ok(std::borrow::Cow::Borrowed("a")))
        ),
        "plain segments are borrowed"
    );
    assert_eq!(path("").segments().count(), 0);
    assert_eq!(path("/").segments().collect::<Vec<_>>(), ["", ""]);
    assert_eq!(path("a/").segments().collect::<Vec<_>>(), ["a", ""]);
    assert_eq!(
        path("/a/%FF").components().nth(2),
        Some(Err(UrlError::Utf8))
    );
    assert_eq!(
        (parsed.to_string(), parsed.as_ref()),
        ("/a/b%2Fc/caf%C3%A9".to_owned(), "/a/b%2Fc/caf%C3%A9")
    );
    assert_eq!(format!("{parsed:?}"), "UrlPath(\"/a/b%2Fc/caf%C3%A9\")");
    assert_eq!("/x".parse::<UrlPath>(), Ok(path("/x")));
}

#[test]
fn compares_with_text_and_borrows_as_text() {
    use std::borrow::Borrow;
    let parsed = path("/a");
    assert!(parsed == "/a" && parsed == *"/a" && parsed != "/b");
    assert_eq!(Borrow::<str>::borrow(&parsed), "/a");
    let set: std::collections::HashSet<UrlPath> = [path("/a"), path("/b")].into_iter().collect();
    assert!(
        set.contains("/a"),
        "lookups by text agree with the path's hash"
    );
    assert!(path("/a") < path("/b"));
}

#[test]
fn builds_paths_from_segments_and_components() {
    assert_eq!(
        UrlPath::from_components(["", "a", "b/c", "d e"]).unwrap(),
        "/a/b%2Fc/d%20e"
    );
    assert_eq!(UrlPath::from_components(["", ""]).unwrap(), "/");
    assert_eq!(
        UrlPath::from_components(Vec::<String>::new()).unwrap(),
        UrlPath::new()
    );
    assert_eq!(
        UrlPath::from_components(["...", ".a", "a."]).unwrap(),
        ".../.a/a.",
        "only `.` and `..` navigate"
    );
    for dot in [".", ".."] {
        assert_eq!(
            UrlPath::from_components(["users", dot]),
            Err(UrlError::Component),
            "a {dot:?} value cannot be data"
        );
    }
    assert_eq!(
        UrlPath::from_segments(["", "a", "b%2Fc", ".."]).unwrap(),
        "/a/b%2Fc/..",
        "encoded segments are syntax"
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
fn inspects_file_names() {
    assert!(path("/a").is_absolute() && !path("a").is_absolute());
    assert!(path("/a/").is_directory() && !path("/a").is_directory());
    assert!(UrlPath::new().is_empty() && UrlPath::default().is_empty());
    let report = path("/docs/My%20Report.tar.gz");
    assert_eq!(report.file_name(), Some(Ok("My Report.tar.gz".into())));
    assert_eq!(report.file_stem(), Some(Ok("My Report.tar".into())));
    assert_eq!(report.extension(), Some(Ok("gz".into())));
    assert!(matches!(
        path("a/report.pdf").file_stem(),
        Some(Ok(std::borrow::Cow::Borrowed("report")))
    ));
    for (name, stem, extension) in [
        (".profile", ".profile", None),
        ("README", "README", None),
        (".profile.local", ".profile", Some("local")),
        ("a.", "a", Some("")),
    ] {
        let parsed = UrlPath::from_components([name]).unwrap();
        assert_eq!(
            (parsed.file_stem(), parsed.extension()),
            (
                Some(Ok(stem.into())),
                extension.map(|extension| Ok(extension.into()))
            ),
            "{name}"
        );
    }
    for no_file in ["", "/", "/docs/", "/a/.", "/a/..", "/a/%2E%2e", "."] {
        assert_eq!(
            path(no_file).file_name(),
            None,
            "{no_file:?} names a directory"
        );
        assert_eq!(
            (path(no_file).file_stem(), path(no_file).extension()),
            (None, None)
        );
    }
    assert_eq!(path("/%FF.txt").file_name(), Some(Err(UrlError::Utf8)));
    assert_eq!(path("/%FF.txt").file_stem(), Some(Err(UrlError::Utf8)));
    assert_eq!(path("/%FF.txt").extension(), Some(Err(UrlError::Utf8)));
}

#[test]
fn directories_contain_the_resources_paths_name() {
    for (original, directory) in [
        ("/docs/report.pdf", "/docs/"),
        ("/docs/guide/", "/docs/guide/"),
        ("/", "/"),
        ("/a", "/"),
        ("/a/b/..", "/a/"),
        ("/a/b/c/../d", "/a/b/"),
        ("a/b", "a/"),
        ("a", "./"),
        ("", "./"),
        ("../a", "../"),
    ] {
        assert_eq!(path(original).directory(), path(directory), "{original:?}");
    }
}

#[test]
fn parents_are_the_level_above_in_directory_form() {
    let cases = [
        ("/docs/report.pdf", Some("/docs/")),
        ("/docs/guide/", Some("/docs/")),
        ("/a", Some("/")),
        ("/a/", Some("/")),
        ("/", None),
        ("/..", None),
        // Dot segments are resolved first, so a parent is never deeper than its path.
        ("/a/b/..", Some("/")),
        ("/a/./b", Some("/a/")),
        // An empty segment is a level, as it is when resolving `..`.
        ("/a//b", Some("/a//")),
        ("/a//", Some("/a/")),
        ("//", Some("/")),
        ("a/b", Some("a/")),
        ("a/b/", Some("a/")),
        ("a", Some("./")),
        ("../a", Some("../")),
        // Above a relative path's own segments depends on its base.
        ("./", None),
        ("../", None),
        ("../..", None),
        ("", None),
        ("a/..//b", Some(".//")),
        (".//", Some("./")),
        ("a//", Some("a/")),
        // Percent-encoded dots are dots here, as a server that decodes them sees them.
        ("/a/b/%2E%2E", Some("/")),
        ("/a/b/%2e", Some("/a/")),
        ("/a/%2E%2E/b", Some("/")),
    ];
    for (child, parent) in cases {
        assert_eq!(path(child).parent(), parent.map(path), "{child:?}");
    }
    // Resolving against a parent stays within it.
    let parent = path("/docs/report.pdf").parent().unwrap();
    assert_eq!(
        parent.resolve(&path("guide.html")),
        path("/docs/guide.html")
    );
}

#[test]
fn ancestors_end_at_the_root_or_the_top_of_a_relative_path() {
    let names = |text: &str| {
        path(text)
            .ancestors()
            .map(|ancestor| ancestor.to_string())
            .collect::<Vec<_>>()
    };
    assert_eq!(names("/a/b/c.txt"), ["/a/b/c.txt", "/a/b/", "/a/", "/"]);
    assert_eq!(names("a/b"), ["a/b", "a/", "./"]);
    assert_eq!(names("../a"), ["../a", "../"]);
    assert_eq!(names("/"), ["/"]);
    assert_eq!(
        names("a//b"),
        ["a//b", "a//", "a/", "./"],
        "an empty segment is a level in relative paths too"
    );
    assert_eq!(
        names("/a/b/.."),
        ["/a/b/..", "/"],
        "the path itself is unchanged"
    );
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
    // Empty segments are kept, unlike `simplify`, including before a final dot segment.
    assert_eq!(path("/a//b").resolve(&path("c")), path("/a//c"));
    assert_eq!(
        path("/a//b").resolve(&path("../c")),
        path("/a/c"),
        "`..` removes the empty segment, as RFC 3986 does"
    );
    assert_eq!(path("/b//c").resolve(&path(".")), path("/b//"));
    assert_eq!(path("/b//c/").resolve(&path("..")), path("/b//"));
    assert_eq!(path("/b/c").resolve(&path(".//.")), path("/b//"));
}

#[test]
fn relative_results_stay_relative_and_name_directories_explicitly() {
    for (base, reference, expected) in [
        ("a/b", "../../c", "../c"),
        ("file", "x", "x"),
        ("a/b", "..", "./"),
        ("a/b", "../", "./"),
        ("a", "./", "./"),
        ("a", ".", "./"),
        ("a/b", "..//c", ".//c"),
        ("", "./", "./"),
    ] {
        assert_eq!(
            path(base).resolve(&path(reference)),
            path(expected),
            "{base} + {reference}"
        );
    }
    assert_eq!(UrlPath::new().join(&path("./")), path("./"));
}

#[test]
fn joins_paths_as_directories() {
    for (base, other, expected) in [
        ("/api/users", "42", "/api/users/42"),
        ("/api/users/", "42", "/api/users/42"),
        ("/api", "../status", "/status"),
        ("/api", "/v2/x", "/v2/x"),
        ("/api/./x", "", "/api/./x"),
        ("", "a/./b", "a/b"),
        ("docs", "../../x", "../x"),
    ] {
        assert_eq!(
            path(base).join(&path(other)),
            path(expected),
            "{base} + {other}"
        );
    }
}

#[test]
fn joined_components_are_always_data() {
    let users = path("/api/users");
    assert_eq!(
        users.join_component("Ada Lovelace"),
        Ok(path("/api/users/Ada%20Lovelace"))
    );
    assert_eq!(
        users.join_component("a/../../b"),
        Ok(path("/api/users/a%2F..%2F..%2Fb"))
    );
    assert_eq!(users.join_component("/etc"), Ok(path("/api/users/%2Fetc")));
    for invalid in ["", ".", ".."] {
        assert_eq!(
            users.join_component(invalid),
            Err(UrlError::Component),
            "{invalid:?}"
        );
    }
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
        // A remainder beginning with an empty segment must not become an absolute or network path.
        ("/a//b", "/a/x", ".//b"),
        ("////evil.com", "/a", ".////evil.com"),
        // Dot segments are removed first.
        ("/a/./b", "/a/x", "b"),
        ("/a/b", "/a/c/../x", "b"),
    ];
    for (target, base, expected) in cases {
        let relative = path(target).relative_to(&path(base));
        assert_eq!(relative, path(expected), "{target} from {base}");
        assert!(!relative.is_absolute());
        assert_eq!(
            path(base).resolve(&relative),
            path("/").resolve(&path(target)),
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
fn writes_relative_paths_explicitly() {
    for (original, explicit) in [
        ("guide.html", "./guide.html"),
        ("a/b", "./a/b"),
        ("./a", "./a"),
        ("../a", "../a"),
        ("/a", "/a"),
        ("", ""),
        (".", "."),
        ("..", ".."),
    ] {
        assert_eq!(path(original).to_explicit(), path(explicit), "{original:?}");
    }
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
        ("/a%5Cb", "a\\b"),
        ("", ""),
        ("/", ""),
    ];
    for (url, local) in cases {
        assert_eq!(path(url).to_local_path(root), Ok(root.join(local)), "{url}");
    }
    for (url, error) in [
        ("/../etc/passwd", LocalPathError::EscapesRoot),
        ("/a/../../b", LocalPathError::EscapesRoot),
        ("/%2E%2E/x", LocalPathError::EscapesRoot),
        ("/folder/safe%2Fname/file.txt", LocalPathError::Component),
        ("/folder/a%2F..%2F..%2Fsecret", LocalPathError::Component),
        ("/a%00b", LocalPathError::Component),
    ] {
        assert_eq!(path(url).to_local_path(root), Err(error), "{url}");
    }
    assert_eq!(
        path("x").to_local_path(PathBuf::from("relative")),
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
    assert_eq!(
        UrlPath::from_local_path("docs/"),
        Ok(path("docs/")),
        "a trailing separator names a directory"
    );
    assert_eq!(UrlPath::from_local_path("./"), Ok(path("./")));
    for (local, url) in [(".", "./"), ("a/.", "a/"), ("a/..", "a/../"), ("..", "../")] {
        assert_eq!(
            UrlPath::from_local_path(local),
            Ok(path(url)),
            "{local:?} names a directory"
        );
    }
    assert_eq!(UrlPath::from_local_path(""), Ok(UrlPath::new()));
    assert_eq!(
        UrlPath::from_local_path("/srv/public"),
        Err(LocalPathError::Absolute)
    );
    let local = Path::new("files/My Document.txt");
    let root = Path::new("/srv/public");
    assert_eq!(
        UrlPath::from_local_path(local).unwrap().to_local_path(root),
        Ok(root.join(local)),
        "the mapping round-trips"
    );
}

#[test]
fn builds_absolute_url_paths_for_files_beneath_a_root() {
    let root = Path::new("/srv/public");
    let cases = [
        (
            "/srv/public/docs/My Report.pdf",
            "/docs/My%20Report.pdf",
            "/srv/public/docs/My Report.pdf",
        ),
        ("/srv/public/docs/", "/docs/", "/srv/public/docs"),
        ("/srv/public", "/", "/srv/public"),
        ("/srv/public/", "/", "/srv/public"),
        ("/srv/public/a/../b", "/b", "/srv/public/b"),
    ];
    for (local, url, mapped) in cases {
        assert_eq!(
            UrlPath::from_local_path_under(root, local),
            Ok(path(url)),
            "{local}"
        );
        assert_eq!(
            path(url).to_local_path(root),
            Ok(PathBuf::from(mapped)),
            "{url} maps back beneath the root"
        );
    }
    assert_eq!(UrlPath::from_local_path_under("", "./a"), Ok(path("/a")));
    for outside in ["/srv/other/x", "/srv/public/../x", "relative"] {
        assert_eq!(
            UrlPath::from_local_path_under(root, outside),
            Err(LocalPathError::EscapesRoot),
            "{outside}"
        );
    }
}

#[cfg(unix)]
#[test]
fn unix_file_names_are_bytes() {
    use std::os::unix::ffi::OsStrExt;
    let root = Path::new("/srv/public");
    let latin1 = std::ffi::OsStr::from_bytes(b"caf\xe9");
    let url = UrlPath::from_local_path(latin1).unwrap();
    assert_eq!(url, "caf%E9");
    assert_eq!(
        url.to_local_path(root),
        Ok(root.join(latin1)),
        "non-UTF-8 names round-trip exactly"
    );
    assert_eq!(
        UrlPath::from_local_path_under(root, root.join(latin1)),
        Ok(path("/caf%E9"))
    );
    assert_eq!(
        UrlPath::from_local_path(std::ffi::OsStr::from_bytes(b"a\0b")),
        Err(LocalPathError::Component),
        "NUL cannot round-trip"
    );
    assert_eq!(
        UrlPath::from_local_path_under(root, root.join(std::ffi::OsStr::from_bytes(b"a\0b"))),
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
            "path escapes the root directory",
            "path component does not correspond to one file name",
            "local path is not relative"
        ]
    );
    let error: Box<dyn std::error::Error> = Box::new(LocalPathError::Component);
    assert!(error.source().is_none());
}

/// A deterministic generator of small paths over the characters that matter to dot-segment removal.
struct Paths(u64);

impl Paths {
    fn next(&mut self) -> String {
        const PARTS: [&str; 7] = ["a", "b", "", ".", "..", "%2E", "c:d"];
        let mut text = String::new();
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let count = (self.0 >> 60) as usize;
        if self.0 >> 59 & 1 == 1 {
            text.push('/');
        }
        for index in 0..count {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            if index > 0 {
                text.push('/');
            }
            text.push_str(PARTS[(self.0 >> 61) as usize % PARTS.len()]);
        }
        text
    }
}

#[test]
fn relative_paths_round_trip_through_resolution() {
    let mut paths = Paths(1);
    for _ in 0..20_000 {
        let (target, base) = (format!("/{}", paths.next()), format!("/{}", paths.next()));
        let (target, base) = (path(&target), path(&base).simplify());
        let relative = target.relative_to(&base);
        assert!(!relative.is_absolute(), "{target} from {base}");
        assert_eq!(
            base.resolve(&relative),
            path("/").resolve(&target),
            "{target} from {base} via {relative}"
        );
    }
}

#[test]
fn parents_terminate_and_never_deepen() {
    let mut paths = Paths(7);
    for _ in 0..20_000 {
        let original = path(&paths.next());
        let directory = original.directory();
        let ancestors: Vec<UrlPath> = original.ancestors().skip(1).take(64).collect();
        assert!(
            ancestors.len() <= original.segments().count() + 1,
            "{original} has {} ancestors",
            ancestors.len()
        );
        for ancestor in &ancestors {
            assert!(ancestor.is_directory(), "{original}: {ancestor}");
            if original.is_absolute() {
                assert!(
                    directory.as_str().starts_with(ancestor.as_str()),
                    "{original}: {ancestor} is within {directory}"
                );
            }
        }
    }
}

#[test]
fn local_paths_agree_with_normalized_and_simplified_paths() {
    // Access decisions made on `normalize().simplify()` see the file that `to_local_path` opens, whenever it opens one. Where `simplify` stops `..` at the root, `to_local_path` refuses instead.
    let root = Path::new("/srv/public");
    let mut paths = Paths(11);
    let mut refused = 0;
    for _ in 0..20_000 {
        let original = path(&format!("/{}", paths.next()));
        let simplified = original.normalize().simplify();
        match original.to_local_path(root) {
            Ok(local) => assert_eq!(simplified.to_local_path(root), Ok(local), "{original}"),
            Err(error) => {
                assert_eq!(error, LocalPathError::EscapesRoot, "{original}");
                refused += 1;
            }
        }
        assert!(
            simplified.to_local_path(root).is_ok(),
            "{simplified} has no dot segments left to escape with"
        );
    }
    assert!(refused > 0, "the generator reaches the root");
    let tricky = path("/public//../private/x");
    assert_eq!(tricky.resolve(&path("")), tricky);
    assert_eq!(
        path("/").resolve(&tricky),
        path("/public/private/x"),
        "RFC 3986 resolution keeps the empty segment as a level"
    );
    assert_eq!(
        tricky.to_local_path(root),
        Ok(root.join("private/x")),
        "the filesystem does not"
    );
    assert_eq!(tricky.normalize().simplify(), path("/private/x"));
}
