# Forms and Query Strings

This guide explains how to read `application/x-www-form-urlencoded` content and query strings with `base-web-url`, whole or as it arrives, and how to read bracketed names such as `user[emails][]`.

## Core concepts

Form content and query strings share one syntax, which the WHATWG URL Standard defines: pairs separated by `&`, each a name and a value separated by `=`, with `+` for a space and `%XX` for other bytes.

- A name ends at its first `=`, so `a=b=c` has the value `b=c`.
- Empty pairs, as in `a=1&&b=2` or a trailing `&`, are skipped.
- A name without `=`, as in `debug` in `?debug&page=2`, has no value, which is distinct from the empty value of `debug=`.
- Decoding is strict: a `%` not followed by two hexadecimal digits, and decoded bytes that are not UTF-8, are errors, never kept as text or replaced.

Errors give the offset in the input where the problem was found, and never include names or values; `Debug` output shows their lengths instead. When input has several problems, the first in the input is reported, however the input is split.

## Reading input held in memory

`form_pairs` gives the pairs of input already in memory, such as a URL's query:

```rust
use base_web_url::{Url, form_pairs};

let url = Url::parse("https://example.com/search?q=fish+%26+chips&page=2&debug")?;
let mut pairs = Vec::new();
for pair in form_pairs(url.query().unwrap_or("")) {
    let pair = pair?;
    pairs.push((pair.name().as_str().to_owned(), pair.value().map(str::to_owned)));
}
assert_eq!(pairs, [
    ("q".to_owned(), Some("fish & chips".to_owned())),
    ("page".to_owned(), Some("2".to_owned())),
    ("debug".to_owned(), None),
]);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Names and values are borrowed from the input when they have no escapes or `+`.

## Reading content as it arrives

`FormParser` reads a body as it arrives, without holding all of it. The caller owns the buffer: each call reports how many bytes it consumed and one event, and the next call begins with the bytes that remain. When the input has ended, call `parse_end` with what remains until it reports `End`, since the last pair ends there:

```rust
use base_web_url::{FormEvent, FormParser};

fn handle(event: FormEvent<'_>, fields: &mut Vec<(String, String)>) {
    match event {
        FormEvent::Name(name) => fields.push((name.as_str().to_owned(), String::new())),
        FormEvent::Value(piece) => fields.last_mut().unwrap().1.push_str(&piece),
        _ => {}
    }
}

let chunks: [&[u8]; 3] = [b"name=Ada+Lo", b"velace&role=engi", b"neer"];
let (mut parser, mut buffer, mut fields) = (FormParser::new(), Vec::new(), Vec::new());
for chunk in chunks {
    buffer.extend_from_slice(chunk);
    loop {
        let parsed = parser.parse(&buffer)?;
        let incomplete = parsed.event == FormEvent::Incomplete;
        handle(parsed.event, &mut fields);
        buffer.drain(..parsed.consumed);
        if incomplete {
            break;
        }
    }
}
loop {
    let parsed = parser.parse_end(&buffer)?;
    let end = parsed.event == FormEvent::End;
    handle(parsed.event, &mut fields);
    buffer.drain(..parsed.consumed);
    if end {
        break;
    }
}
assert_eq!(fields, [("name".to_owned(), "Ada Lovelace".to_owned()), ("role".to_owned(), "engineer".to_owned())]);
# Ok::<(), base_web_url::FormError>(())
```

Each pair is reported as `Name`, any number of `Value` pieces, and `PairEnd`. Once `parse_end` has been called, the input has ended, and bytes given to later calls are never read. The name is always complete, so an application can refuse a name it does not expect before reading the value. A value arrives in pieces, decoded, so it never needs to be held whole; a piece never ends within an escape or a UTF-8 sequence. The buffer must be able to hold `minimum_buffer_size()` bytes, enough for a whole name; the parser reports an error rather than needing more.

## Limits

`FormLimits` bounds what hostile input can make the parser hold or count: the size of the input, 2 MiB by default; the number of pairs, 1024; the length of a name, 1 KiB, which the parser holds whole; the length of a value, by default the size limit, since values stream; and the depth of a bracket path, 8. Lengths count the bytes as encoded in the input. An application applies its own limits on a field's decoded value as the pieces arrive.

## Bracket paths

`FormName::path` reads a name as a bracket path, as Rails and PHP applications send nested fields. The path is given as segments, for the application to map onto its own structures:

```rust
use base_web_url::{FormPathSegment, form_pairs};

let pair = form_pairs("items[][name]=Widget").next().unwrap()?;
let path = pair.name().path()?;
let segments: Vec<_> = path.segments().collect();
assert_eq!(segments, [FormPathSegment::Key("items"), FormPathSegment::Element, FormPathSegment::Key("name")]);
# Ok::<(), base_web_url::FormError>(())
```

A path is a key followed by any number of `[key]` and `[]`, where `[]` is an element of an array whose position the application decides. Keys are not empty and contain neither `[` nor `]`; anything else, such as `[a]`, `a[b`, or `a[b]c`, is an error rather than a guess, since other readers disagree about such names. Brackets are read after decoding, so `a%5Bb%5D` is the path `a`, `b`. Paths deeper than the depth limit are errors.

`FormPath::parse` reads a name from elsewhere, such as a `multipart/form-data` field's, by the same rules.

## Writing

There is no form writer: to write a query or form content, encode each name and value with `encode_component`, and join them with `=` and `&`. It writes a space as `%20`, which readers decode as they decode `+`. Mapping pairs onto application data, such as declared structures and arrays, belongs to `base-web-content`.
