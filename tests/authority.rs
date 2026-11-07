// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_web_url::{Authority, UrlError, decode_component, encode_component};

fn authority(text: &str) -> Authority {
    Authority::parse(text).unwrap()
}

#[test]
fn parses_userinfo_hosts_and_ports() {
    let parsed = authority("user:p%40ss@example.com:8080");
    assert_eq!(
        (parsed.userinfo(), parsed.host(), parsed.port()),
        (Some("user:p%40ss"), "example.com", Some(8080))
    );
    assert_eq!(
        (parsed.as_str(), parsed.to_string()),
        (
            "user:p%40ss@example.com:8080",
            "user:p%40ss@example.com:8080".to_owned()
        )
    );
    assert_eq!(
        format!("{parsed:?}"),
        "Authority(\"user:p%40ss@example.com:8080\")"
    );
    let cases = [
        ("example.com", None, "example.com", None),
        ("192.0.2.1:443", None, "192.0.2.1", Some(443)),
        ("[2001:db8::1]:8443", None, "[2001:db8::1]", Some(8443)),
        ("[::ffff:192.0.2.1]", None, "[::ffff:192.0.2.1]", None),
        ("[v1.fe80::a+en1]", None, "[v1.fe80::a+en1]", None),
        ("", None, "", None),
        ("example.com:", None, "example.com", None),
        ("@example.com", Some(""), "example.com", None),
        ("ex%41mple.com:0", None, "ex%41mple.com", Some(0)),
    ];
    for (text, userinfo, host, port) in cases {
        let parsed = authority(text);
        assert_eq!(
            (parsed.userinfo(), parsed.host(), parsed.port()),
            (userinfo, host, port),
            "{text:?}"
        );
    }
    assert_eq!(
        "example.com".parse::<Authority>(),
        Ok(authority("example.com"))
    );
}

#[test]
fn rejects_invalid_authorities() {
    let cases = [
        ("exa mple.com", UrlError::Host),
        ("user@host@example.com", UrlError::Host),
        ("us er@example.com", UrlError::Host),
        ("[2001:db8::1", UrlError::Host),
        ("[2001:db8::g]", UrlError::Host),
        ("[192.0.2.1]", UrlError::Host),
        ("[v1]", UrlError::Host),
        ("[v.a]", UrlError::Host),
        ("[vx.a]", UrlError::Host),
        ("[v1.]", UrlError::Host),
        ("[v1.a/b]", UrlError::Host),
        ("[::1]x", UrlError::Host),
        ("example.com:65536", UrlError::Port),
        ("example.com:-1", UrlError::Port),
        ("example.com:8a", UrlError::Port),
        ("ex%4", UrlError::PercentEncoding),
    ];
    for (text, error) in cases {
        assert_eq!(Authority::parse(text), Err(error), "{text:?}");
    }
}

#[test]
fn encodes_and_decodes_components() {
    assert_eq!(encode_component("Az09-._~"), "Az09-._~");
    assert_eq!(
        encode_component("a b/c?d&e=f#g%h+i"),
        "a%20b%2Fc%3Fd%26e%3Df%23g%25h%2Bi"
    );
    assert_eq!(
        encode_component("caf\u{e9} \u{2764}"),
        "caf%C3%A9%20%E2%9D%A4"
    );
    assert_eq!(
        decode_component("caf%C3%A9%20%e2%9d%a4+x").unwrap(),
        "caf\u{e9} \u{2764}+x"
    );
    assert!(matches!(
        decode_component("plain"),
        Ok(std::borrow::Cow::Borrowed("plain"))
    ));
    for malformed in ["%", "%4", "%4g", "a%zz"] {
        assert_eq!(
            decode_component(malformed),
            Err(UrlError::PercentEncoding),
            "{malformed:?}"
        );
    }
    assert_eq!(decode_component("%FF"), Err(UrlError::Utf8));
    for value in ["", "a/b c", "\u{1f600}%", "100%"] {
        assert_eq!(
            decode_component(&encode_component(value)).unwrap(),
            value,
            "{value:?} round-trips"
        );
    }
}

#[test]
fn errors_describe_the_problem() {
    let errors = [
        UrlError::Syntax,
        UrlError::Scheme,
        UrlError::Host,
        UrlError::Port,
        UrlError::PercentEncoding,
        UrlError::Utf8,
        UrlError::Relative,
        UrlError::Path,
    ];
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(
        messages,
        [
            "invalid character in URL",
            "invalid URL scheme",
            "invalid URL authority",
            "invalid URL port",
            "invalid percent-encoding in URL",
            "percent-decoded URL component is not UTF-8",
            "URL has no scheme",
            "URL path is incompatible with its other components"
        ]
    );
    let error: Box<dyn std::error::Error> = Box::new(UrlError::Syntax);
    assert!(error.source().is_none());
}
