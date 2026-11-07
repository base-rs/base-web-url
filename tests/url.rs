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
        "Url(\"https://example.com/docs/?page=2#top\")"
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
            .join(&UrlPath::from_components(["users", "a/b"]))
            .as_str(),
        "https://example.com/v1/users/a%2Fb"
    );
    // Without an authority, a joined path beginning with `//` is kept from being read as one.
    assert_eq!(url("foo:/a").join(&path("/..//x")).as_str(), "foo:/.//x");
}

#[test]
fn errors_propagate_from_each_component() {
    assert_eq!(Url::parse("https://exa mple.com/"), Err(UrlError::Host));
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
