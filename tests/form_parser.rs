// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_web_url::{
    FormError, FormErrorKind, FormEvent, FormLimits, FormParser, FormPathSegment, form_pairs,
};
use std::borrow::Cow;

/// Names and values, without offsets.
type Pairs = Vec<(String, Option<String>)>;

/// Each pair as the events describe it: its name's offset, its name, and its value reassembled from every piece.
type Collected = Vec<(u64, String, Option<String>)>;

fn apply(event: FormEvent<'_>, pairs: &mut Collected, open: &mut bool) {
    match event {
        FormEvent::Name(name) => {
            assert!(!*open, "a pair begins only after the previous one ends");
            *open = true;
            pairs.push((
                name.offset(),
                name.as_str().to_owned(),
                name.has_value().then(String::new),
            ));
        }
        FormEvent::Value(piece) => {
            assert!(*open && !piece.is_empty());
            let value = pairs.last_mut().unwrap().2.as_mut();
            value.expect("a value follows `=`").push_str(&piece);
        }
        FormEvent::PairEnd => {
            assert!(*open);
            *open = false;
        }
        FormEvent::End => assert!(!*open),
        FormEvent::Incomplete => {}
    }
}

/// Feed `chunks` through a parser as a caller would: append each to a buffer, parse until more input is needed, and drop what was consumed; then parse the rest as the end of the input.
fn collect(limits: FormLimits, chunks: &[&[u8]]) -> Result<Collected, FormError> {
    let mut parser = FormParser::with_limits(limits);
    let (mut buffer, mut pairs, mut open) = (Vec::new(), Collected::new(), false);
    for chunk in chunks {
        buffer.extend_from_slice(chunk);
        loop {
            let parsed = parser.parse(&buffer)?;
            let (consumed, incomplete) = (parsed.consumed, parsed.event == FormEvent::Incomplete);
            assert_ne!(parsed.event, FormEvent::End);
            assert!(
                !incomplete || consumed > 0 || buffer.len() < parser.minimum_buffer_size(),
                "the parser makes progress with its minimum buffer"
            );
            apply(parsed.event, &mut pairs, &mut open);
            buffer.drain(..consumed);
            if incomplete {
                break;
            }
        }
    }
    loop {
        let parsed = parser.parse_end(&buffer)?;
        let (consumed, done) = (parsed.consumed, parsed.event == FormEvent::End);
        assert_ne!(parsed.event, FormEvent::Incomplete);
        apply(parsed.event, &mut pairs, &mut open);
        buffer.drain(..consumed);
        if done {
            break;
        }
    }
    assert!(buffer.is_empty() && parser.is_finished());
    Ok(pairs)
}

/// The pairs `form_pairs` gives, in the same form.
fn in_memory(limits: FormLimits, input: &[u8]) -> Result<Collected, FormError> {
    base_web_url::FormPairs::with_limits(input, limits)
        .map(|pair| {
            let pair = pair?;
            Ok((
                pair.name().offset(),
                pair.name().as_str().to_owned(),
                pair.value().map(str::to_owned),
            ))
        })
        .collect()
}

/// Every way of splitting the input in two, and one byte at a time, gives the same result as the whole input at once, as does `form_pairs`.
fn assert_split_invariant(limits: FormLimits, input: &[u8]) -> Result<Collected, FormError> {
    let expected = collect(limits, &[input]);
    for split in 0..=input.len() {
        let (first, second) = input.split_at(split);
        assert_eq!(
            collect(limits, &[first, second]),
            expected,
            "split at {split}"
        );
    }
    let bytes: Vec<&[u8]> = input.chunks(1).collect();
    assert_eq!(collect(limits, &bytes), expected, "a byte at a time");
    assert_eq!(in_memory(limits, input), expected, "in memory");
    expected
}

fn parse(input: &[u8]) -> Result<Collected, FormError> {
    assert_split_invariant(FormLimits::default(), input)
}

fn pairs(input: &[u8]) -> Pairs {
    parse(input)
        .unwrap()
        .into_iter()
        .map(|(_, name, value)| (name, value))
        .collect()
}

fn pair(name: &str, value: Option<&str>) -> (String, Option<String>) {
    (name.to_owned(), value.map(str::to_owned))
}

fn error(input: &[u8]) -> (FormErrorKind, u64) {
    let error = parse(input).unwrap_err();
    (error.kind(), error.offset())
}

fn limited_error(limits: FormLimits, input: &[u8]) -> (FormErrorKind, u64) {
    let error = assert_split_invariant(limits, input).unwrap_err();
    (error.kind(), error.offset())
}

#[test]
fn parses_names_and_values() {
    assert_eq!(
        pairs(b"name=Ada+Lovelace&email=ada%40example.com&note=1%2B1%3D2%20%26%20more"),
        [
            pair("name", Some("Ada Lovelace")),
            pair("email", Some("ada@example.com")),
            pair("note", Some("1+1=2 & more")),
        ]
    );
    assert_eq!(
        parse(b"a=1&bb=22&c%20d=3").unwrap(),
        [
            (0, "a".to_owned(), Some("1".to_owned())),
            (4, "bb".to_owned(), Some("22".to_owned())),
            (10, "c d".to_owned(), Some("3".to_owned())),
        ]
    );
}

#[test]
fn values_end_at_the_next_ampersand_and_keep_later_equals_signs() {
    assert_eq!(
        pairs(b"a=b=c&d==&e%3Df=g"),
        [
            pair("a", Some("b=c")),
            pair("d", Some("=")),
            pair("e=f", Some("g"))
        ]
    );
}

#[test]
fn skips_empty_pairs() {
    assert_eq!(
        pairs(b"&&a=1&&b=2&"),
        [pair("a", Some("1")), pair("b", Some("2"))]
    );
    assert!(pairs(b"").is_empty());
    assert!(pairs(b"&&&").is_empty());
}

#[test]
fn distinguishes_names_without_values_from_empty_values() {
    assert_eq!(
        pairs(b"a&b=&c&=d&="),
        [
            pair("a", None),
            pair("b", Some("")),
            pair("c", None),
            pair("", Some("d")),
            pair("", Some("")),
        ]
    );
    assert_eq!(pairs(b"flag"), [pair("flag", None)]);
}

#[test]
fn decodes_utf8_text() {
    assert_eq!(
        pairs("caf%C3%A9=na%C3%AFve&city=Z\u{fc}rich&sign=%E2%82%AC+%F0%9F%A6%80".as_bytes()),
        [
            pair("café", Some("naïve")),
            pair("city", Some("Zürich")),
            pair("sign", Some("€ 🦀")),
        ]
    );
    // A byte order mark and control characters are data.
    assert_eq!(
        pairs(b"%EF%BB%BFa=%00%0D%0A"),
        [pair("\u{feff}a", Some("\0\r\n"))]
    );
}

#[test]
fn borrows_text_without_escapes() {
    let input = b"name=value&a+b=%41";
    let mut parser = FormParser::new();
    let name = match parser.parse(input).unwrap().event {
        FormEvent::Name(name) => name,
        event => panic!("{event:?}"),
    };
    assert!(matches!(name.clone().into_text(), Cow::Borrowed("name")));
    let parsed = parser.parse(&input[5..]).unwrap();
    assert_eq!(parsed.event, FormEvent::Value(Cow::Borrowed("value")));
    let parsed = parser.parse(&input[10..]).unwrap();
    assert_eq!(parsed.event, FormEvent::PairEnd);
    let FormEvent::Name(name) = parser.parse(&input[11..]).unwrap().event else {
        panic!("a name follows");
    };
    assert!(matches!(name.into_text(), Cow::Owned(text) if text == "a b"));
}

#[test]
fn rejects_malformed_percent_encoding() {
    assert_eq!(error(b"a=%"), (FormErrorKind::PercentEncoding, 2));
    assert_eq!(error(b"a=%4"), (FormErrorKind::PercentEncoding, 2));
    assert_eq!(error(b"a=1%4&b=2"), (FormErrorKind::PercentEncoding, 3));
    assert_eq!(error(b"a=%G1"), (FormErrorKind::PercentEncoding, 2));
    assert_eq!(error(b"a=%1G"), (FormErrorKind::PercentEncoding, 2));
    assert_eq!(error(b"a=%%41"), (FormErrorKind::PercentEncoding, 2));
    assert_eq!(error(b"b%=1"), (FormErrorKind::PercentEncoding, 1));
    assert_eq!(error(b"ok=1&b%"), (FormErrorKind::PercentEncoding, 6));
    assert_eq!(error(b"%zz"), (FormErrorKind::PercentEncoding, 0));
    // A `+` is data, never part of an escape.
    assert_eq!(error(b"a=%+1"), (FormErrorKind::PercentEncoding, 2));
}

#[test]
fn rejects_text_that_is_not_utf8() {
    assert_eq!(error(b"a=%FF"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=\xff"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=x%C3"), (FormErrorKind::Utf8, 3));
    assert_eq!(error(b"a=x%C3&b=1"), (FormErrorKind::Utf8, 3));
    assert_eq!(error(b"a=%C3x"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=\xc3"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=%E2%82"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=%E2\x82\xac%E2"), (FormErrorKind::Utf8, 7));
    // Overlong encodings, surrogates, and code points beyond U+10FFFF.
    assert_eq!(error(b"a=%C0%AF"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=%ED%A0%80"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=%F4%90%80%80"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"%FF=1"), (FormErrorKind::Utf8, 0));
    assert_eq!(error(b"\xc3&a"), (FormErrorKind::Utf8, 0));
    assert_eq!(error(b"a\xc3"), (FormErrorKind::Utf8, 1));
}

#[test]
fn reports_errors_in_the_order_of_the_input() {
    assert_eq!(error(b"a=\xff%G"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=%FF%G"), (FormErrorKind::Utf8, 2));
    assert_eq!(error(b"a=%C3%G"), (FormErrorKind::PercentEncoding, 5));
    assert_eq!(error(b"a=\xc3%"), (FormErrorKind::PercentEncoding, 3));
    assert_eq!(error(b"\xff%G=1"), (FormErrorKind::Utf8, 0));
    assert_eq!(error(b"%C3%=1"), (FormErrorKind::PercentEncoding, 3));
}

#[test]
fn limits_the_size_of_the_input() {
    let limits = |size_limit| FormLimits {
        size_limit,
        ..FormLimits::default()
    };
    assert_eq!(
        assert_split_invariant(limits(7), b"a=1&b=2").unwrap().len(),
        2
    );
    assert_eq!(
        limited_error(limits(6), b"a=1&b=2"),
        (FormErrorKind::SizeLimit, 6)
    );
    assert_eq!(
        limited_error(limits(4), b"a=1&&&"),
        (FormErrorKind::SizeLimit, 4)
    );
    assert_eq!(
        limited_error(limits(5), b"a=%C3%A9"),
        (FormErrorKind::SizeLimit, 5)
    );
    assert_eq!(
        limited_error(limits(2), b"abc=1"),
        (FormErrorKind::SizeLimit, 2)
    );
    assert_eq!(
        limited_error(limits(0), b"a"),
        (FormErrorKind::SizeLimit, 0)
    );
    // Errors are reported in the order of the input, so a malformed name within the limit fails as one.
    assert_eq!(
        limited_error(limits(2), b"%ZZ"),
        (FormErrorKind::PercentEncoding, 0)
    );
    assert_eq!(
        limited_error(limits(3), b"\xffaaaa"),
        (FormErrorKind::Utf8, 0)
    );
    assert_eq!(
        limited_error(limits(5), b"%ZZaaaaaaa=1"),
        (FormErrorKind::PercentEncoding, 0)
    );
    assert_eq!(
        limited_error(limits(6), b"a=1&b%C3%A9"),
        (FormErrorKind::SizeLimit, 6)
    );
    assert!(assert_split_invariant(limits(0), b"").unwrap().is_empty());
    // Bytes within the limit are reported before the error.
    let mut parser = FormParser::with_limits(limits(4));
    let parsed = parser.parse(b"a=12345").unwrap();
    assert!(parsed.consumed == 2 && matches!(parsed.event, FormEvent::Name(_)));
    assert_eq!(
        parser.parse(b"12345").unwrap().event,
        FormEvent::Value(Cow::Borrowed("12"))
    );
    assert_eq!(
        parser.parse(b"345").unwrap_err().kind(),
        FormErrorKind::SizeLimit
    );
}

#[test]
fn limits_the_number_of_pairs() {
    let limits = FormLimits {
        pair_count_limit: 2,
        ..FormLimits::default()
    };
    assert_eq!(assert_split_invariant(limits, b"a=1&&b&").unwrap().len(), 2);
    assert_eq!(
        limited_error(limits, b"a=1&&b&cdef=3"),
        (FormErrorKind::PairCountLimit, 7)
    );
    assert_eq!(
        limited_error(
            FormLimits {
                pair_count_limit: 0,
                ..limits
            },
            b"&a"
        ),
        (FormErrorKind::PairCountLimit, 1)
    );
}

#[test]
fn limits_the_length_of_names() {
    let limits = FormLimits {
        name_length_limit: 3,
        ..FormLimits::default()
    };
    assert_eq!(
        assert_split_invariant(limits, b"abc=1&%41&abc")
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        limited_error(limits, b"a=1&abcd=1"),
        (FormErrorKind::NameLengthLimit, 7)
    );
    assert_eq!(
        limited_error(limits, b"abcd"),
        (FormErrorKind::NameLengthLimit, 3)
    );
    assert_eq!(
        limited_error(limits, b"%41%42=1"),
        (FormErrorKind::NameLengthLimit, 3)
    );
    let empty = FormLimits {
        name_length_limit: 0,
        ..limits
    };
    assert_eq!(assert_split_invariant(empty, b"=1&=2").unwrap().len(), 2);
    assert_eq!(
        limited_error(empty, b"=1&a"),
        (FormErrorKind::NameLengthLimit, 3)
    );
    // Errors are reported in the order of the input, so a malformed name within the limit fails as one.
    assert_eq!(limited_error(limits, b"%FF%FF"), (FormErrorKind::Utf8, 0));
    assert_eq!(
        limited_error(
            FormLimits {
                name_length_limit: 4,
                ..limits
            },
            b"%ZZabcdef=1"
        ),
        (FormErrorKind::PercentEncoding, 0)
    );
    assert_eq!(
        limited_error(limits, b"ab%C3%A9"),
        (FormErrorKind::NameLengthLimit, 3)
    );
}

#[test]
fn limits_the_length_of_values() {
    let limits = FormLimits {
        value_length_limit: 3,
        ..FormLimits::default()
    };
    assert_eq!(
        pairs_with(limits, b"a=abc&b=%41&c=&d"),
        [
            pair("a", Some("abc")),
            pair("b", Some("A")),
            pair("c", Some("")),
            pair("d", None)
        ]
    );
    assert_eq!(
        limited_error(limits, b"a=abcd"),
        (FormErrorKind::ValueLengthLimit, 5)
    );
    assert_eq!(
        limited_error(limits, b"a=%41%42"),
        (FormErrorKind::ValueLengthLimit, 5)
    );
    assert_eq!(
        limited_error(limits, b"a=a%41"),
        (FormErrorKind::ValueLengthLimit, 5)
    );
    assert_eq!(
        limited_error(limits, b"a=ab%C3%A9"),
        (FormErrorKind::ValueLengthLimit, 5)
    );
    // Within the limit, a malformed escape fails as one.
    assert_eq!(
        limited_error(limits, b"a=a%4&"),
        (FormErrorKind::PercentEncoding, 3)
    );
    let empty = FormLimits {
        value_length_limit: 0,
        ..limits
    };
    assert_eq!(
        pairs_with(empty, b"a=&b"),
        [pair("a", Some("")), pair("b", None)]
    );
    assert_eq!(
        limited_error(empty, b"a=&b=1"),
        (FormErrorKind::ValueLengthLimit, 5)
    );
}

fn pairs_with(limits: FormLimits, input: &[u8]) -> Pairs {
    assert_split_invariant(limits, input)
        .unwrap()
        .into_iter()
        .map(|(_, name, value)| (name, value))
        .collect()
}

#[test]
fn limits_do_not_overflow() {
    let limits = FormLimits {
        size_limit: usize::MAX,
        pair_count_limit: usize::MAX,
        name_length_limit: usize::MAX,
        value_length_limit: usize::MAX,
        depth_limit: usize::MAX,
    };
    assert_eq!(
        pairs_with(limits, b"a[b]=1&c"),
        [pair("a[b]", Some("1")), pair("c", None)]
    );
    let parser = FormParser::with_limits(limits);
    assert_eq!(parser.minimum_buffer_size(), usize::MAX);
    assert_eq!(parser.limits(), limits);
}

#[test]
fn needs_at_most_its_minimum_buffer() {
    assert_eq!(
        FormParser::new().minimum_buffer_size(),
        FormLimits::NAME_LENGTH_LIMIT + 1
    );
    let small = FormParser::with_limits(FormLimits {
        name_length_limit: 2,
        ..FormLimits::default()
    });
    assert_eq!(small.minimum_buffer_size(), 12);
    // The longest piece of a value the parser may leave for later: an escaped incomplete UTF-8 sequence and a partial escape.
    let mut parser = small;
    let input = b"a=%F0%9F%A6%8";
    let parsed = parser.parse(input).unwrap();
    assert_eq!(parsed.consumed, 2);
    assert_eq!(
        parser.parse(&input[2..]).unwrap().event,
        FormEvent::Incomplete
    );
    assert_eq!(
        parser.parse(b"%F0%9F%A6%80").unwrap().event,
        FormEvent::Value(Cow::Owned("🦀".to_owned()))
    );
}

#[test]
fn reports_the_same_error_again() {
    let mut parser = FormParser::new();
    assert_eq!(parser.parse(b"a=%G").unwrap().consumed, 2);
    let error = parser.parse(b"%G").unwrap_err();
    assert_eq!(parser.parse(b"b=1").unwrap_err(), error);
    assert_eq!(parser.parse_end(b"").unwrap_err(), error);
    assert!(!parser.is_finished());
    assert_eq!(
        error.to_string(),
        "invalid percent-encoding in form content at byte 2"
    );
}

#[test]
fn reports_the_end_again() {
    let mut parser = FormParser::default();
    assert_eq!(parser.parse(b"").unwrap().event, FormEvent::Incomplete);
    assert_eq!(parser.parse_end(b"").unwrap().event, FormEvent::End);
    assert!(parser.is_finished());
    assert_eq!(parser.parse(b"").unwrap().event, FormEvent::End);
    assert_eq!(parser.parse_end(b"").unwrap().event, FormEvent::End);
}

#[test]
fn never_reads_beyond_the_end_of_the_input() {
    let mut parser = FormParser::new();
    let parsed = parser.parse_end(b"a").unwrap();
    assert_eq!(parsed.consumed, 1);
    // Bytes given after the end are not part of the input, and `parse` is `parse_end` once the input has ended.
    let parsed = parser.parse(b"xyz").unwrap();
    assert_eq!((parsed.consumed, parsed.event), (0, FormEvent::PairEnd));
    let parsed = parser.parse(b"xyz").unwrap();
    assert_eq!((parsed.consumed, parsed.event), (0, FormEvent::End));
    let parsed = parser.parse_end(b"more=1").unwrap();
    assert_eq!((parsed.consumed, parsed.event), (0, FormEvent::End));

    let mut parser = FormParser::new();
    assert!(matches!(
        parser.parse_end(b"a=12").unwrap().event,
        FormEvent::Name(_)
    ));
    let parsed = parser.parse(b"12345").unwrap();
    assert_eq!(parsed.event, FormEvent::Value(Cow::Borrowed("12")));
    assert_eq!(parser.parse(b"").unwrap().event, FormEvent::PairEnd);
}

#[test]
fn names_compare_by_what_they_show() {
    let name = |limits| {
        let mut pairs = base_web_url::FormPairs::with_limits("a[b]=1", limits);
        pairs.next().unwrap().unwrap().into_parts().0
    };
    let shallow = name(FormLimits {
        depth_limit: 1,
        ..FormLimits::default()
    });
    assert_eq!(shallow, name(FormLimits::default()));
    assert_ne!(
        shallow,
        form_pairs("a[b]").next().unwrap().unwrap().into_parts().0
    );
}

#[test]
fn long_input_fed_a_byte_at_a_time() {
    // The parser searches each byte a bounded number of times, so this takes linear time.
    let length = 256 * 1024;
    let limits = FormLimits {
        name_length_limit: length,
        ..FormLimits::default()
    };
    let mut input = vec![b'n'; length];
    input.push(b'=');
    input.extend(std::iter::repeat_n(b'v', length));
    let bytes: Vec<&[u8]> = input.chunks(1).collect();
    let pairs = collect(limits, &bytes).unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].1.len(), length);
    assert_eq!(pairs[0].2.as_ref().unwrap().len(), length);
}

#[test]
fn describes_errors_without_content() {
    let descriptions = [
        (
            b"a=%FF&secret".as_slice(),
            FormLimits::default(),
            "form content is not UTF-8 at byte 2",
        ),
        (
            b"secret=1",
            FormLimits {
                size_limit: 3,
                ..FormLimits::default()
            },
            "form content exceeds its size limit at byte 3",
        ),
        (
            b"a&secret",
            FormLimits {
                pair_count_limit: 1,
                ..FormLimits::default()
            },
            "form content exceeds its pair count limit at byte 2",
        ),
        (
            b"secret",
            FormLimits {
                name_length_limit: 3,
                ..FormLimits::default()
            },
            "form name exceeds its length limit at byte 3",
        ),
        (
            b"a=secret",
            FormLimits {
                value_length_limit: 3,
                ..FormLimits::default()
            },
            "form value exceeds its length limit at byte 5",
        ),
    ];
    for (input, limits, description) in descriptions {
        let error = collect(limits, &[input]).unwrap_err();
        assert_eq!(error.to_string(), description);
        assert!(!format!("{error:?}").contains("secret"));
    }
    let names = form_pairs("secret=1&a[secret");
    let errors: Vec<String> = names
        .skip(1)
        .map(|pair| pair.unwrap().name().path().unwrap_err().to_string())
        .collect();
    assert_eq!(errors, ["form name is not a bracket path at byte 9"]);
    let deep = form_pairs("a[b][c]").next().unwrap().unwrap();
    let limited = base_web_url::FormPairs::with_limits(
        "a[b][c]",
        FormLimits {
            depth_limit: 2,
            ..FormLimits::default()
        },
    )
    .next()
    .unwrap()
    .unwrap();
    assert_eq!(deep.name().path().unwrap().depth(), 3);
    assert_eq!(
        limited.name().path().unwrap_err().to_string(),
        "form name exceeds its depth limit at byte 0"
    );
}

#[test]
fn debug_output_hides_names_and_values() {
    let mut parser = FormParser::new();
    let input = b"secret=password&a=%70assword";
    let parsed = parser.parse(input).unwrap();
    let debug = format!("{parsed:?}");
    assert!(!debug.contains("secret"), "{debug}");
    assert!(debug.contains("<6 bytes>"), "{debug}");
    let value = parser.parse(&input[7..]).unwrap();
    assert_eq!(format!("{:?}", value.event), "Value(<8 bytes>)");
    assert_eq!(format!("{:?}", FormEvent::PairEnd), "PairEnd");
    assert_eq!(format!("{:?}", FormEvent::End), "End");
    assert_eq!(format!("{:?}", FormEvent::Incomplete), "Incomplete");
    assert!(!format!("{parser:?}").contains("secret"));

    let mut pairs = form_pairs(&input[..]);
    assert_eq!(
        format!("{pairs:?}"),
        "FormPairs { position: 0, length: 28 }"
    );
    let first = pairs.next().unwrap().unwrap();
    let debug = format!("{first:?}");
    assert_eq!(
        debug,
        "FormPair { name: FormName { text: <6 bytes>, has_value: true, offset: 0 }, value: Some(<8 bytes>) }"
    );
    let flag = form_pairs("secret").next().unwrap().unwrap();
    assert!(format!("{flag:?}").ends_with("value: None }"));
    let (name, value) = first.into_parts();
    assert_eq!(
        (name.as_str(), value.as_deref()),
        ("secret", Some("password"))
    );

    let path = form_pairs("user[secret][]").next().unwrap().unwrap();
    let path = path.name().path().unwrap();
    assert_eq!(format!("{path:?}"), "FormPath(<3 segments>)");
    let segments = path.segments();
    assert_eq!(format!("{segments:?}"), "FormPathSegments(<14 bytes>)");
    let debug: Vec<String> = segments.map(|segment| format!("{segment:?}")).collect();
    assert_eq!(debug, ["Key(<4 bytes>)", "Key(<6 bytes>)", "Element"]);
    assert!(path.segments().eq([
        FormPathSegment::Key("user"),
        FormPathSegment::Key("secret"),
        FormPathSegment::Element
    ]));
}

#[test]
fn in_memory_pairs_stop_after_an_error() {
    let mut pairs = form_pairs(b"a=1&b=%G&c=3");
    assert_eq!(pairs.next().unwrap().unwrap().value(), Some("1"));
    let error = pairs.next().unwrap().unwrap_err();
    assert_eq!(
        (error.kind(), error.offset()),
        (FormErrorKind::PercentEncoding, 6)
    );
    assert!(pairs.next().is_none());
    assert!(pairs.next().is_none());
    let mut empty = form_pairs("");
    assert!(empty.next().is_none() && empty.next().is_none());
}

#[test]
fn split_invariance_for_varied_input() {
    let inputs: [&[u8]; 12] = [
        b"a=1&b=2&c=3",
        b"&&name=Ada+Lovelace&&",
        b"x=%E2%82%AC%F0%9F%A6%80&y=caf\xc3\xa9",
        b"k%C3%A9y=v%C3%A1lue&%26=%3D",
        b"flag&empty=&=nameless",
        b"a=%E2%82",
        b"a=b%4",
        b"a=\xe2\x82",
        b"a=1&b=%FF",
        b"user[name]=Ada&user[emails][]=a%40b&tags[]=x",
        b"a=%F0%9F%A6%80%F0%9F%A6",
        b"=",
    ];
    for input in inputs {
        let _ = parse(input);
        let tight = FormLimits {
            size_limit: 20,
            pair_count_limit: 2,
            name_length_limit: 4,
            value_length_limit: 5,
            depth_limit: 2,
        };
        let _ = assert_split_invariant(tight, input);
    }
}

/// A small deterministic generator for property tests.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        (self.next() % bound as u64) as usize
    }
}

/// The WHATWG URL Standard's `application/x-www-form-urlencoded` parser, transcribed directly, with strict decoding: a malformed escape, or decoded bytes that are not UTF-8, are errors, reported in the order of the input. A name without `=` has no value.
fn reference(input: &[u8]) -> Result<Pairs, (FormErrorKind, u64)> {
    let mut output = Vec::new();
    let mut start = 0;
    for sequence in input.split(|&byte| byte == b'&') {
        let offset = start;
        start += sequence.len() + 1;
        if sequence.is_empty() {
            continue;
        }
        let (name, value) = match sequence.iter().position(|&byte| byte == b'=') {
            Some(index) => (
                &sequence[..index],
                Some((&sequence[index + 1..], offset + index + 1)),
            ),
            None => (sequence, None),
        };
        let name = reference_decode(name, offset)?;
        let value = value
            .map(|(value, offset)| reference_decode(value, offset))
            .transpose()?;
        output.push((name, value));
    }
    Ok(output)
}

/// Replace `+` with a space and percent-decode, then decode UTF-8, as the standard does, failing instead of keeping a malformed escape or replacing invalid UTF-8.
fn reference_decode(encoded: &[u8], offset: usize) -> Result<String, (FormErrorKind, u64)> {
    let mut bytes = Vec::new();
    let mut origins = Vec::new();
    let mut index = 0;
    let mut malformed = None;
    while index < encoded.len() {
        origins.push(index);
        match encoded[index] {
            b'+' => bytes.push(b' '),
            b'%' => match encoded.get(index + 1..index + 3) {
                Some(digits) if digits.iter().all(u8::is_ascii_hexdigit) => {
                    let digits = std::str::from_utf8(digits).unwrap();
                    bytes.push(u8::from_str_radix(digits, 16).unwrap());
                    index += 2;
                }
                _ => {
                    origins.pop();
                    malformed = Some(index);
                    break;
                }
            },
            byte => bytes.push(byte),
        }
        index += 1;
    }
    let at = |index: usize| (offset + index) as u64;
    match (std::str::from_utf8(&bytes), malformed) {
        (Ok(text), None) => Ok(text.to_owned()),
        // A sequence cut short by the malformed escape is not itself an error before it.
        (Err(error), Some(index)) if error.error_len().is_none() => {
            Err((FormErrorKind::PercentEncoding, at(index)))
        }
        (Ok(_), Some(index)) => Err((FormErrorKind::PercentEncoding, at(index))),
        (Err(error), _) => Err((FormErrorKind::Utf8, at(origins[error.valid_up_to()]))),
    }
}

#[test]
fn agrees_with_the_standard_for_generated_input() {
    let pieces: [&[u8]; 27] = [
        b"a",
        b"b",
        b"=",
        b"=",
        b"&",
        b"&",
        b"+",
        b"%",
        b"4",
        b"1",
        b"G",
        b"%41",
        b"%2B",
        b"%26",
        b"%3D",
        b"%C3%A9",
        "é".as_bytes(),
        b"%E2%82%AC",
        b"%F0%9F%A6%80",
        b"%C3",
        b"\xff",
        b"\xe2\x82",
        "🦀".as_bytes(),
        b"\xf0%9F%A6\x80",
        b"%F0\x9f",
        b"[",
        b"]",
    ];
    let mut random = Random(0x9e37_79b9_7f4a_7c15);
    let mut outcomes = [0; 3];
    for _ in 0..3000 {
        let length = random.below(16);
        let input: Vec<u8> = (0..length)
            .flat_map(|_| pieces[random.below(pieces.len())].iter().copied())
            .collect();
        let expected = reference(&input);
        outcomes[match &expected {
            Ok(pairs) if pairs.is_empty() => 0,
            Ok(_) => 1,
            Err(_) => 2,
        }] += 1;
        let strip = |result: Result<Collected, FormError>| {
            result
                .map(|pairs| {
                    pairs
                        .into_iter()
                        .map(|(_, name, value)| (name, value))
                        .collect::<Vec<_>>()
                })
                .map_err(|error| (error.kind(), error.offset()))
        };
        assert_eq!(
            strip(in_memory(FormLimits::default(), &input)),
            expected,
            "{input:?}"
        );
        let mut chunks = Vec::new();
        let mut rest = &input[..];
        while !rest.is_empty() {
            let (chunk, after) = rest.split_at(random.below(rest.len()) + 1);
            chunks.push(chunk);
            rest = after;
        }
        assert_eq!(
            strip(collect(FormLimits::default(), &chunks)),
            expected,
            "{input:?} as {chunks:?}"
        );
        // The same result however the input is split, also when tight limits stop it.
        let limits = FormLimits {
            size_limit: random.below(48),
            pair_count_limit: random.below(4),
            name_length_limit: random.below(12),
            value_length_limit: random.below(16),
            depth_limit: FormLimits::DEPTH_LIMIT,
        };
        let _ = assert_split_invariant(limits, &input);
        // Random pieces, under limits that may be zero, small, or the largest.
        let mut limit = |small: usize| match random.below(4) {
            0 => usize::MAX,
            1 => usize::MAX - random.below(3),
            _ => random.below(small),
        };
        let limits = FormLimits {
            size_limit: limit(64),
            pair_count_limit: limit(5),
            name_length_limit: limit(14),
            value_length_limit: limit(18),
            depth_limit: FormLimits::DEPTH_LIMIT,
        };
        assert_eq!(
            collect(limits, &chunks),
            collect(limits, &[&input]),
            "{input:?} as {chunks:?} with {limits:?}"
        );
    }
    assert!(outcomes.iter().all(|&count| count > 100), "{outcomes:?}");
}
