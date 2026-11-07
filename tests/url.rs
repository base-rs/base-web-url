// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_web_url::{Authority, Url, UrlError, UrlPath, UrlReference};

fn url(text: &str) -> Url {
    Url::parse(text).unwrap()
}

fn path(text: &str) -> UrlPath {
    UrlPath::parse(text).unwrap()
}

#[test]
fn urls_have_a_scheme() {
    let parsed = url("HTTPS://example.com/docs/?page=2#top");
    assert_eq!(parsed.scheme(), "https");
    assert_eq!(parsed.authority().unwrap().host(), "example.com");
    assert_eq!(parsed.path().as_str(), "/docs/");
    assert_eq!(
        (parsed.query(), parsed.fragment()),
        (Some("page=2"), Some("top"))
    );
    assert_eq!(
        (parsed.as_str(), parsed.to_string()),
        (
            "https://example.com/docs/?page=2#top",
            "https://example.com/docs/?page=2#top".to_owned()
        )
    );
    assert_eq!(
        format!("{parsed:?}"),
        "Url(https://example.com/docs/?page=2#top)"
    );
    assert_eq!(Url::parse("/docs/"), Err(UrlError::Relative));
    assert_eq!(
        "mailto:ada@example.com"
            .parse::<Url>()
            .unwrap()
            .path()
            .as_str(),
        "ada@example.com"
    );
}

#[test]
fn urls_convert_to_and_from_references() {
    let parsed = url("https://example.com/a");
    let as_reference: UrlReference = parsed.clone().into();
    assert_eq!(as_reference, *parsed.as_reference());
    assert_eq!(
        AsRef::<UrlReference>::as_ref(&parsed),
        parsed.as_reference()
    );
    assert_eq!(Url::try_from(as_reference), Ok(parsed));
    assert_eq!(
        Url::try_from(UrlReference::parse("a").unwrap()),
        Err(UrlError::Relative)
    );
}

#[test]
fn builds_urls_from_components() {
    let built = Url::new(
        "HTTPS",
        Some(Authority::parse("example.com").unwrap()),
        path("/docs/"),
    )
    .unwrap();
    assert_eq!(built.as_str(), "https://example.com/docs/");
    assert_eq!(Url::new("1x", None, UrlPath::new()), Err(UrlError::Scheme));
    assert_eq!(
        Url::new(
            "https",
            Some(Authority::parse("example.com").unwrap()),
            path("docs")
        ),
        Err(UrlError::Path)
    );
    let changed = built
        .with_query(Some("page=2"))
        .unwrap()
        .with_fragment(Some("top"))
        .unwrap();
    assert_eq!(changed.as_str(), "https://example.com/docs/?page=2#top");
    let moved = changed
        .clone()
        .with_path(path("/api"))
        .unwrap()
        .with_authority(Some(Authority::parse("api.example.com:8443").unwrap()))
        .unwrap();
    assert_eq!(
        moved.as_str(),
        "https://api.example.com:8443/api?page=2#top"
    );
    assert_eq!(
        changed.clone().with_path(path("relative")),
        Err(UrlError::Path)
    );
    assert_eq!(
        changed.clone().with_query(Some("a b")),
        Err(UrlError::Syntax)
    );
    assert_eq!(
        changed.clone().with_fragment(Some("%")),
        Err(UrlError::PercentEncoding)
    );
    assert_eq!(
        changed.with_authority(None).unwrap().as_str(),
        "https:/docs/?page=2#top"
    );
}

#[test]
fn resolves_links_against_a_page() {
    let page = url("https://example.com/docs/guide/index.html?version=2");
    let cases = [
        ("../api/", "https://example.com/docs/api/"),
        ("style.css", "https://example.com/docs/guide/style.css"),
        (
            "#top",
            "https://example.com/docs/guide/index.html?version=2#top",
        ),
        ("//cdn.example.com/x.js", "https://cdn.example.com/x.js"),
        ("http://other.example/", "http://other.example/"),
    ];
    for (link, expected) in cases {
        assert_eq!(
            page.resolve(UrlReference::parse(link).unwrap()).as_str(),
            expected,
            "{link}"
        );
    }
    assert_eq!(
        page.resolve(url("https://example.org/")).as_str(),
        "https://example.org/"
    );
}

#[test]
fn joins_paths_as_directories() {
    let api = url("https://api.example.com/v1?key=x#a");
    assert_eq!(
        api.join(&path("users")).as_str(),
        "https://api.example.com/v1/users",
        "the query and fragment belong to the original resource"
    );
    assert_eq!(
        api.join(&path("users/../groups/")).as_str(),
        "https://api.example.com/v1/groups/"
    );
    assert_eq!(
        api.join(&path("/status")).as_str(),
        "https://api.example.com/status"
    );
    assert_eq!(
        url("https://example.com").join(&path("a")).as_str(),
        "https://example.com/a"
    );
    assert_eq!(
        url("https://example.com/v1/")
            .join(&UrlPath::from_components(["users", "a/b"]).unwrap())
            .as_str(),
        "https://example.com/v1/users/a%2Fb"
    );
    // Without an authority, a joined path beginning with `//` is kept from being read as one.
    assert_eq!(url("foo:/a").join(&path("/..//x")).as_str(), "foo:/.//x");
}

#[test]
fn errors_propagate_from_each_component() {
    assert_eq!(
        Url::parse("https://exa mple.com/"),
        Err(UrlError::Authority)
    );
    let doubled = url("https://example.com//x");
    assert_eq!(doubled.path().as_str(), "//x");
    assert_eq!(
        doubled.with_authority(None),
        Err(UrlError::Path),
        "`//x` cannot follow no authority"
    );
    assert_eq!(
        Url::new(
            "https",
            Some(Authority::parse("example.com").unwrap()),
            path("//x")
        )
        .unwrap()
        .as_str(),
        "https://example.com//x"
    );
    assert_eq!(
        Url::new("https", None, path("a:b")).unwrap().as_str(),
        "https:a:b",
        "a path with a scheme is kept as given"
    );
    assert_eq!(Url::new("https", None, path("//x")), Err(UrlError::Path));
}

#[test]
fn changes_schemes_and_appends_components() {
    assert_eq!(
        url("http://example.com/a")
            .with_scheme("HTTPS")
            .unwrap()
            .as_str(),
        "https://example.com/a"
    );
    assert_eq!(
        url("http://example.com/a").with_scheme("1x"),
        Err(UrlError::Scheme)
    );
    let api = url("https://api.example.com/v1?key=x");
    assert_eq!(
        api.join_component("users")
            .unwrap()
            .join_component("..")
            .map(|url| url.to_string()),
        Err(UrlError::Component)
    );
    assert_eq!(
        api.join_component("a/../../b").unwrap().as_str(),
        "https://api.example.com/v1/a%2F..%2F..%2Fb"
    );
    assert_eq!(api.join_component(""), Err(UrlError::Component));
}

#[test]
fn urls_compare_and_order_with_references() {
    let parsed = url("https://example.com/a");
    assert!(parsed == UrlReference::parse("https://example.com/a").unwrap());
    assert!(UrlReference::parse("https://example.com/a").unwrap() == parsed);
    assert!(url("https://a.example/") < url("https://b.example/"));
    assert!(Authority::parse("a.example").unwrap() < Authority::parse("b.example").unwrap());
    assert_eq!(
        Authority::parse("a")
            .unwrap()
            .partial_cmp(&Authority::parse("a").unwrap()),
        Some(std::cmp::Ordering::Equal)
    );
}

#[test]
fn urls_relative_to_a_page_keep_their_query() {
    let page = url("https://example.com/docs/guide/index.html");
    assert_eq!(
        url("https://example.com/api/?page=2#x")
            .relative_to(&page)
            .to_string(),
        "../../api/?page=2#x"
    );
    assert_eq!(
        page.resolve(url("https://example.com/api/?page=2#x").relative_to(&page))
            .as_str(),
        "https://example.com/api/?page=2#x"
    );
    assert_eq!(
        url("https://other.example/x")
            .relative_to(&page)
            .to_string(),
        "https://other.example/x",
        "another authority cannot be relative"
    );
}

#[test]
fn normalizes_equivalent_urls_to_one_spelling() {
    let cases = [
        (
            "HTTPS://EXAMPLE.com:443/a/./b/../%7Ec?%7e#%7E",
            "https://example.com/a/~c?~#~",
        ),
        ("http://example.com:80", "http://example.com/"),
        ("http://example.com:8080", "http://example.com:8080/"),
        ("ws://example.com:80/x", "ws://example.com/x"),
        ("wss://example.com:443/x", "wss://example.com/x"),
        ("https://example.com:80/x", "https://example.com:80/x"),
        ("ftp://EXAMPLE.com:21", "ftp://example.com:21"),
        (
            "https://ex%61mple.com/%2e%2E/%2F",
            "https://example.com/%2F",
        ),
        ("foo:a/%2E/../b", "foo:/b"),
        ("mailto:Ada@Example.com", "mailto:Ada@Example.com"),
    ];
    for (original, normalized) in cases {
        assert_eq!(url(original).normalize().as_str(), normalized, "{original}");
        assert_eq!(
            url(normalized).normalize().as_str(),
            normalized,
            "{normalized} is already normal"
        );
    }
    assert_eq!(
        UrlReference::parse("a/./%7e/../b:c?x")
            .unwrap()
            .normalize()
            .to_string(),
        "a/b:c?x"
    );
    assert_eq!(
        UrlReference::parse("x/../a:b")
            .unwrap()
            .normalize()
            .to_string(),
        "./a:b"
    );
}

#[test]
fn debugging_output_hides_passwords() {
    let secret = url("https://user:secret@example.com/a");
    assert_eq!(format!("{secret:?}"), "Url(https://user:***@example.com/a)");
    assert_eq!(
        format!("{:?}", secret.as_reference()),
        "UrlReference(https://user:***@example.com/a)"
    );
    assert_eq!(
        secret.as_str(),
        "https://user:secret@example.com/a",
        "the URL itself is exact"
    );
}
