// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_web_url::{Authority, UrlError, UrlPath, UrlReference};

fn reference(text: &str) -> UrlReference {
    UrlReference::parse(text).unwrap()
}

#[test]
fn parses_each_component() {
    let parsed = reference("HTTPS://user@example.com:8080/docs/a%20b?page=2&q=x#results");
    assert_eq!(parsed.scheme(), Some("https"));
    assert_eq!(
        parsed.authority().map(Authority::as_str),
        Some("user@example.com:8080")
    );
    assert_eq!(parsed.path().as_str(), "/docs/a%20b");
    assert_eq!(
        (parsed.query(), parsed.fragment()),
        (Some("page=2&q=x"), Some("results"))
    );
    assert!(!parsed.is_relative());
    assert_eq!(
        parsed.to_string(),
        "https://user@example.com:8080/docs/a%20b?page=2&q=x#results"
    );
    assert_eq!(
        format!("{parsed:?}"),
        "UrlReference(https://user@example.com:8080/docs/a%20b?page=2&q=x#results)"
    );
}

#[test]
fn keeps_empty_and_absent_components_distinct() {
    for text in [
        "",
        "/",
        "?",
        "#",
        "/search?",
        "/search#",
        "//example.com",
        "file:///etc/hosts",
        "mailto:ada@example.com",
        "urn:isbn:0451450523",
        "a/b:c",
        "?q=a:b/c?d",
        "#a/b?c",
    ] {
        assert_eq!(reference(text).to_string(), text, "{text:?} round-trips");
    }
    assert_eq!(reference("/search?").query(), Some(""));
    assert_eq!(reference("/search").query(), None);
    assert_eq!(
        reference("//example.com").authority().unwrap().host(),
        "example.com"
    );
    assert!(reference("").is_relative() && reference("").path().is_empty());
    assert_eq!("../x".parse::<UrlReference>(), Ok(reference("../x")));
}

#[test]
fn follows_rfc_3986_exactly() {
    let cases = [
        ("a b", UrlError::Syntax),
        ("/a|b", UrlError::Syntax),
        ("/caf\u{e9}", UrlError::Syntax),
        ("/a?b c", UrlError::Syntax),
        ("/a#b#c", UrlError::Syntax),
        ("/a%2", UrlError::PercentEncoding),
        ("/a%zz", UrlError::PercentEncoding),
        ("/a?%", UrlError::PercentEncoding),
        ("1http://example.com", UrlError::Scheme),
        ("ht_tp://example.com", UrlError::Scheme),
        ("http://exa mple.com", UrlError::Authority),
        ("http://[::1", UrlError::Authority),
        ("http://example.com:80a", UrlError::Port),
        // Without a scheme, a `:` in the first segment is ambiguous.
        (":a", UrlError::Path),
        (":a/b", UrlError::Path),
    ];
    for (text, error) in cases {
        assert_eq!(UrlReference::parse(text), Err(error), "{text:?}");
    }
}

#[test]
fn builders_validate_their_components() {
    let built = UrlReference::default()
        .with_scheme(Some("HTTP"))
        .unwrap()
        .with_authority(Some(Authority::parse("example.com").unwrap()))
        .unwrap()
        .with_path(UrlPath::parse("/a").unwrap())
        .unwrap()
        .with_query(Some("q=1"))
        .unwrap()
        .with_fragment(Some("top"))
        .unwrap();
    assert_eq!(built.to_string(), "http://example.com/a?q=1#top");
    assert_eq!(built.clone().with_scheme(Some("1x")), Err(UrlError::Scheme));
    assert_eq!(
        built.clone().with_path(UrlPath::parse("relative").unwrap()),
        Err(UrlError::Path),
        "a path after an authority must be absolute"
    );
    assert_eq!(built.clone().with_query(Some("a b")), Err(UrlError::Syntax));
    assert_eq!(
        built.clone().with_fragment(Some("%")),
        Err(UrlError::PercentEncoding)
    );
    assert_eq!(
        built
            .clone()
            .with_query(None)
            .unwrap()
            .with_fragment(None)
            .unwrap()
            .to_string(),
        "http://example.com/a"
    );
    let without_authority = built.clone().with_authority(None).unwrap();
    assert_eq!(without_authority.to_string(), "http:/a?q=1#top");
    assert_eq!(
        without_authority
            .clone()
            .with_path(UrlPath::parse("//a").unwrap()),
        Err(UrlError::Path),
        "`//` would begin an authority"
    );
    let colon = UrlReference::default()
        .with_path(UrlPath::parse("b").unwrap())
        .unwrap()
        .with_scheme(Some("a"))
        .unwrap();
    assert_eq!(
        colon
            .with_path(UrlPath::parse("x:y").unwrap())
            .unwrap()
            .with_scheme(None),
        Err(UrlError::Path),
        "removing the scheme would leave `x:` looking like one"
    );
}

#[test]
fn paths_become_unambiguous_references() {
    for (path, expected) in [("a:b", "./a:b"), ("//a", "/.//a"), ("a/b", "a/b"), ("", "")] {
        assert_eq!(
            UrlReference::from(UrlPath::parse(path).unwrap()).to_string(),
            expected,
            "{path}"
        );
        assert_eq!(reference(expected).path().as_str(), expected);
    }
}

/// RFC 3986 section 5.4, against the base `http://a/b/c/d;p?q`.
const RESOLUTION_EXAMPLES: [(&str, &str); 42] = [
    ("g:h", "g:h"),
    ("g", "http://a/b/c/g"),
    ("./g", "http://a/b/c/g"),
    ("g/", "http://a/b/c/g/"),
    ("/g", "http://a/g"),
    ("//g", "http://g"),
    ("?y", "http://a/b/c/d;p?y"),
    ("g?y", "http://a/b/c/g?y"),
    ("#s", "http://a/b/c/d;p?q#s"),
    ("g#s", "http://a/b/c/g#s"),
    ("g?y#s", "http://a/b/c/g?y#s"),
    (";x", "http://a/b/c/;x"),
    ("g;x", "http://a/b/c/g;x"),
    ("g;x?y#s", "http://a/b/c/g;x?y#s"),
    ("", "http://a/b/c/d;p?q"),
    (".", "http://a/b/c/"),
    ("./", "http://a/b/c/"),
    ("..", "http://a/b/"),
    ("../", "http://a/b/"),
    ("../g", "http://a/b/g"),
    ("../..", "http://a/"),
    ("../../", "http://a/"),
    ("../../g", "http://a/g"),
    ("../../../g", "http://a/g"),
    ("../../../../g", "http://a/g"),
    ("/./g", "http://a/g"),
    ("/../g", "http://a/g"),
    ("g.", "http://a/b/c/g."),
    (".g", "http://a/b/c/.g"),
    ("g..", "http://a/b/c/g.."),
    ("..g", "http://a/b/c/..g"),
    ("./../g", "http://a/b/g"),
    ("./g/.", "http://a/b/c/g/"),
    ("g/./h", "http://a/b/c/g/h"),
    ("g/../h", "http://a/b/c/h"),
    ("g;x=1/./y", "http://a/b/c/g;x=1/y"),
    ("g;x=1/../y", "http://a/b/c/y"),
    ("g?y/./x", "http://a/b/c/g?y/./x"),
    ("g?y/../x", "http://a/b/c/g?y/../x"),
    ("g#s/./x", "http://a/b/c/g#s/./x"),
    ("g#s/../x", "http://a/b/c/g#s/../x"),
    ("http:g", "http:g"),
];

#[test]
fn resolves_the_rfc_3986_examples() {
    let base = reference("http://a/b/c/d;p?q");
    for (relative, expected) in RESOLUTION_EXAMPLES {
        assert_eq!(
            base.resolve(reference(relative)).to_string(),
            expected,
            "{relative:?}"
        );
    }
}

#[test]
fn resolution_handles_edge_cases() {
    // A base with an authority and an empty path behaves as if its path were `/`.
    assert_eq!(
        reference("http://a").resolve(reference("g")).to_string(),
        "http://a/g"
    );
    assert_eq!(
        reference("http://a").resolve(reference("/g")).to_string(),
        "http://a/g"
    );
    assert_eq!(
        reference("http://a?q").resolve(reference("")).to_string(),
        "http://a?q"
    );
    // Without an authority, a resolved path beginning with `//` gains a `/.` prefix so it is not read as an authority.
    assert_eq!(
        reference("foo:/a").resolve(reference("/..//x")).to_string(),
        "foo:/.//x"
    );
    // Relative bases give relative results, keeping `..` segments they cannot resolve.
    assert_eq!(
        reference("docs/guide/")
            .resolve(reference("../../../api"))
            .to_string(),
        "../api"
    );
    assert_eq!(
        reference("docs/").resolve(reference("../a:b")).to_string(),
        "./a:b"
    );
    assert_eq!(
        reference("/docs/")
            .resolve(reference("//cdn.example.com/x"))
            .to_string(),
        "//cdn.example.com/x"
    );
}

#[test]
fn removing_an_authority_checks_the_path() {
    assert_eq!(
        reference("//example.com//x").with_authority(None),
        Err(UrlError::Path)
    );
}

/// A formatter destination that fails after accepting `remaining` characters.
struct Failing {
    remaining: usize,
}

impl std::fmt::Write for Failing {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        for _ in text.chars() {
            self.remaining = self.remaining.checked_sub(1).ok_or(std::fmt::Error)?;
        }
        Ok(())
    }
}

#[test]
fn formatting_errors_propagate() {
    use std::fmt::Write as _;
    let full = reference("https://example.com/a?q#f");
    let length = full.to_string().len();
    for remaining in 0..length {
        assert!(
            write!(Failing { remaining }, "{full}").is_err(),
            "{remaining}"
        );
    }
    assert!(write!(Failing { remaining: length }, "{full}").is_ok());
}

#[test]
fn rootless_paths_with_a_scheme_follow_rfc_3986_exactly() {
    assert_eq!(
        reference("foo:a/b").resolve(reference("../c")).to_string(),
        "foo:/c"
    );
    assert_eq!(
        reference("foo:a").resolve(reference(".")).to_string(),
        "foo:"
    );
    assert_eq!(
        reference("foo:").resolve(reference(".")).to_string(),
        "foo:",
        "resolution is idempotent"
    );
    assert_eq!(
        reference("urn:a:b").resolve(reference("c")).to_string(),
        "urn:c"
    );
}

#[test]
fn references_relative_to_a_base() {
    let base = reference("https://example.com/docs/index.html");
    let cases = [
        ("/api/?q#f", "../api/?q#f"),
        ("https://example.com/docs/a", "a"),
        ("//example.com/docs/", "./"),
        ("https://other.example/x", "https://other.example/x"),
        ("//other.example/x", "//other.example/x"),
        ("http://example.com/x", "http://example.com/x"),
        ("x", "x"),
        ("/a:b", "../a:b"),
        ("/docs/a:b", "./a:b"),
    ];
    for (target, expected) in cases {
        let relative = reference(target).relative_to(&base);
        assert_eq!(relative.to_string(), expected, "{target}");
    }
    assert_eq!(
        reference("/x")
            .relative_to(reference("https://example.com"))
            .to_string(),
        "/x",
        "a base without an absolute path is not comparable"
    );
    assert_eq!(
        base.resolve(reference("/api/?q#f").relative_to(&base))
            .to_string(),
        "https://example.com/api/?q#f"
    );
}

#[test]
fn joins_and_writes_references_explicitly() {
    assert_eq!(
        reference("docs/?q#f")
            .join(&UrlPath::parse("a").unwrap())
            .to_string(),
        "docs/a"
    );
    assert_eq!(
        reference("//example.com")
            .join(&UrlPath::parse("a").unwrap())
            .to_string(),
        "//example.com/a"
    );
    for (original, explicit) in [
        ("guide.html?q", "./guide.html?q"),
        ("../a", "../a"),
        ("/a", "/a"),
        ("?q", "?q"),
        ("https:a", "https:a"),
        ("//h/a", "//h/a"),
    ] {
        assert_eq!(
            reference(original).to_explicit().to_string(),
            explicit,
            "{original}"
        );
    }
}

/// RFC 3986 section 5.2.4, transcribed literally, as an independent check.
// Rules A and B of the RFC strip prefixes of the same length; they stay separate to match the text.
#[allow(clippy::if_same_then_else)]
fn rfc_remove_dot_segments(mut input: &str) -> String {
    let mut output = String::new();
    while !input.is_empty() {
        if input.starts_with("../") {
            input = &input[3..];
        } else if input.starts_with("./") {
            input = &input[2..];
        } else if input.starts_with("/./") {
            input = &input[2..];
        } else if input == "/." {
            input = "/";
        } else if input.starts_with("/../") {
            input = &input[3..];
            output.truncate(output.rfind('/').unwrap_or(0));
        } else if input == "/.." {
            input = "/";
            output.truncate(output.rfind('/').unwrap_or(0));
        } else if input == "." || input == ".." {
            input = "";
        } else {
            let start = usize::from(input.starts_with('/'));
            let end = input[start..]
                .find('/')
                .map_or(input.len(), |index| index + start);
            output.push_str(&input[..end]);
            input = &input[end..];
        }
    }
    output
}

/// RFC 3986 section 5.2.2 and 5.2.3, transcribed literally, for a base with a scheme.
fn rfc_resolve(base: &UrlReference, reference: &UrlReference) -> String {
    let (scheme, authority, path, query);
    if reference.scheme().is_some() {
        (scheme, authority, path, query) = (
            reference.scheme(),
            reference.authority(),
            rfc_remove_dot_segments(reference.path().as_str()),
            reference.query(),
        );
    } else if reference.authority().is_some() {
        (scheme, authority, path, query) = (
            base.scheme(),
            reference.authority(),
            rfc_remove_dot_segments(reference.path().as_str()),
            reference.query(),
        );
    } else {
        (scheme, authority) = (base.scheme(), base.authority());
        if reference.path().is_empty() {
            (path, query) = (base.path().to_string(), reference.query().or(base.query()));
        } else if reference.path().is_absolute() {
            (path, query) = (
                rfc_remove_dot_segments(reference.path().as_str()),
                reference.query(),
            );
        } else {
            let merged = if base.authority().is_some() && base.path().is_empty() {
                format!("/{}", reference.path())
            } else {
                let base_path = base.path().as_str();
                format!(
                    "{}{}",
                    &base_path[..base_path.rfind('/').map_or(0, |index| index + 1)],
                    reference.path()
                )
            };
            (path, query) = (rfc_remove_dot_segments(&merged), reference.query());
        }
    }
    // A path without an authority that begins with `//` is written with a `/.` prefix, as `UrlReference` documents.
    let path = if authority.is_none() && path.starts_with("//") {
        format!("/.{path}")
    } else {
        path
    };
    let mut text = format!("{}:", scheme.unwrap());
    if let Some(authority) = authority {
        text.push_str(&format!("//{authority}"));
    }
    text.push_str(&path);
    if let Some(query) = query {
        text.push_str(&format!("?{query}"));
    }
    if let Some(fragment) = reference.fragment() {
        text.push_str(&format!("#{fragment}"));
    }
    text
}

#[test]
fn resolution_matches_a_literal_transcription_of_rfc_3986() {
    const PARTS: [&str; 8] = ["a", "b", "", ".", "..", "g;x", "%2E", "c:d"];
    let mut state: u64 = 3;
    let mut next = |modulus: u64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) % modulus
    };
    let bases = [
        "http://a/b/c/d;p?q",
        "http://a/b//c",
        "http://a",
        "foo:a/b",
        "foo:a",
        "foo:/a/b/",
        "urn:x:y",
        "file:///a/b",
    ];
    for base in bases {
        let base = reference(base);
        for _ in 0..20_000 {
            let mut text = String::new();
            match next(4) {
                0 => text.push('/'),
                1 => text.push_str("//h/"),
                _ => {}
            }
            for index in 0..next(5) {
                if index > 0 {
                    text.push('/');
                }
                text.push_str(PARTS[next(PARTS.len() as u64) as usize]);
            }
            if next(4) == 0 {
                text.push_str("?y");
            }
            let Ok(relative) = UrlReference::parse(&text) else {
                continue;
            };
            assert_eq!(
                base.resolve(&relative).to_string(),
                rfc_resolve(&base, &relative),
                "{base} + {text}"
            );
        }
    }
}
