// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use FormPathSegment::{Element, Key};
use base_web_url::{FormErrorKind, FormLimits, FormPairs, FormPath, FormPathSegment, form_pairs};

/// A segment, owned: a key, or `None` for `[]`.
type Segment = Option<String>;

/// The segments of `name`, given after another pair, so that errors have the offset 8.
fn segments(name: &str) -> Result<Vec<Segment>, (FormErrorKind, u64)> {
    let input = format!("ignored&{}=1", base_web_url::encode_component(name));
    let pair = form_pairs(&input).nth(1).unwrap().unwrap();
    let path = pair
        .name()
        .path()
        .map_err(|error| (error.kind(), error.offset()))?;
    let segments: Vec<Segment> = path
        .segments()
        .map(|segment| match segment {
            Key(key) => Some(key.to_owned()),
            Element => None,
        })
        .collect();
    assert_eq!(segments.len(), path.depth());
    Ok(segments)
}

fn key(text: &str) -> Segment {
    Some(text.to_owned())
}

const ELEMENT: Segment = None;

#[test]
fn reads_keys_and_elements() {
    assert_eq!(segments("name").unwrap(), [key("name")]);
    assert_eq!(segments("user[name]").unwrap(), [key("user"), key("name")]);
    assert_eq!(segments("tags[]").unwrap(), [key("tags"), ELEMENT]);
    assert_eq!(
        segments("items[][name]").unwrap(),
        [key("items"), ELEMENT, key("name")]
    );
    assert_eq!(
        segments("a[b][c][][0]").unwrap(),
        [key("a"), key("b"), key("c"), ELEMENT, key("0")]
    );
    assert_eq!(
        segments("matrix[][]").unwrap(),
        [key("matrix"), ELEMENT, ELEMENT]
    );
    // Keys are any other text, including spaces and `=`.
    assert_eq!(
        segments("a b[c=d][é]").unwrap(),
        [key("a b"), key("c=d"), key("é")]
    );
}

#[test]
fn reads_brackets_after_decoding() {
    let pair = form_pairs("a%5Bb%5D%5B%5D=1").next().unwrap().unwrap();
    let path = pair.name().path().unwrap();
    assert!(path.segments().eq([Key("a"), Key("b"), Element]));
}

#[test]
fn rejects_names_that_are_not_bracket_paths() {
    for name in [
        "", "[a]", "[]", "a[b", "a[", "a[b]c", "a]", "a]b", "a[b]]", "a[[b]]", "a[b[c]]", "a[b][",
        "a[b]x[c]",
    ] {
        assert_eq!(
            segments(name),
            Err((FormErrorKind::InvalidPath, 8)),
            "{name:?}"
        );
    }
}

#[test]
fn limits_the_depth_of_paths() {
    let eight = "a[b][c][d][e][f][g][h]";
    assert_eq!(segments(eight).unwrap().len(), FormLimits::DEPTH_LIMIT);
    assert_eq!(
        segments(&format!("{eight}[]")),
        Err((FormErrorKind::DepthLimit, 8))
    );
    let limits = FormLimits {
        depth_limit: 1,
        ..FormLimits::default()
    };
    let mut pairs = FormPairs::with_limits("a=1&a[b]=2", limits);
    assert_eq!(
        pairs
            .next()
            .unwrap()
            .unwrap()
            .name()
            .path()
            .unwrap()
            .depth(),
        1
    );
    let error = pairs.next().unwrap().unwrap().name().path().unwrap_err();
    assert_eq!(
        (error.kind(), error.offset()),
        (FormErrorKind::DepthLimit, 4)
    );
    let none = FormLimits {
        depth_limit: 0,
        ..limits
    };
    let pair = FormPairs::with_limits("a", none).next().unwrap().unwrap();
    assert_eq!(
        pair.name().path().unwrap_err().kind(),
        FormErrorKind::DepthLimit
    );
}

#[test]
fn segments_iterate_once() {
    let pair = form_pairs("a[b][]").next().unwrap().unwrap();
    let path = pair.name().path().unwrap();
    let mut segments = path.segments();
    assert_eq!(segments.next(), Some(Key("a")));
    assert_eq!(segments.clone().count(), 2);
    assert_eq!(segments.next(), Some(Key("b")));
    assert_eq!(segments.next(), Some(Element));
    assert_eq!(segments.next(), None);
    assert_eq!(segments.next(), None);
    assert_eq!(path, pair.name().path().unwrap());
}

#[test]
fn reads_names_from_elsewhere_by_the_same_rules() {
    let path = FormPath::parse("files[][name]", FormLimits::DEPTH_LIMIT).unwrap();
    assert!(path.segments().eq([Key("files"), Element, Key("name")]));
    assert_eq!(
        path,
        form_pairs("files[][name]")
            .next()
            .unwrap()
            .unwrap()
            .name()
            .path()
            .unwrap()
    );
    assert_eq!(FormPath::parse("[a]", 8), Err(FormErrorKind::InvalidPath));
    assert_eq!(FormPath::parse("a[b]", 1), Err(FormErrorKind::DepthLimit));
    assert_eq!(FormPath::parse("a", 1).unwrap().depth(), 1);
}
