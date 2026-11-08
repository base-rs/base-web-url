# Form Content

## Requirement

`FormParser` must parse `application/x-www-form-urlencoded` content, and query strings, which use the same syntax, from bytes fed in pieces of any size, and must give the same events, values, and error however the input is split. The caller owns the buffer: each call reports the bytes it consumed and one event, and the next call's input begins with the bytes not consumed. A pair's complete decoded name, with whether a value follows, must be reported before any of its value, so that a consumer can refuse a name before reading its value; the value is reported as decoded pieces, borrowed from the input when they have no escapes, so a consumer never has to hold a whole value or body. The parser must need at most `minimum_buffer_size()` bytes of input to make progress or report an error, must not search the same bytes more than a bounded number of times however the input is split, and must be told when the input has ended, because the last pair ends there.

The syntax follows the WHATWG URL Standard's `application/x-www-form-urlencoded` parser:

- the input is split at every `&`, and empty sequences, as in `a=1&&b=2` or a leading or trailing `&`, are skipped and are not pairs;
- a pair's name is the text before its first `=`, and its value is everything after it, including any further `=`;
- `+` decodes to a space and `%XX` to its byte, in names and in values, after splitting, so `%26`, `%3D`, and `%2B` are data;
- a sequence without `=` is a name without a value, which is distinct from an empty value, as in `a=`; the WHATWG parser gives both an empty value.

Decoding is strict where the WHATWG parser is lenient: a `%` not followed by two hexadecimal digits is an error rather than being kept as text, and decoded bytes that are not UTF-8 are an error rather than becoming replacement characters. There is no other character set.

`FormName::path` must read a decoded name as a bracket path, `key *( "[" [ key ] "]" )`, where a key is not empty and contains neither `[` nor `]`, giving its segments in order, each a key or `[]`, an array element whose position the consumer decides. Anything else, including an empty name, `[a]`, `a[b`, `a[b]c`, and `a]`, is an error, as is a path of more segments, the first included, than the depth limit. The path is given as segments, not assembled into a structure. Brackets are read after decoding, so `a%5Bb%5D` is the path `a`, `b`.

The parser must apply limits on the input's total size, the number of pairs, the length of each name and each value as encoded in the input, and the depth of a path, with defaults of 2 MiB, 1024, 1 KiB, the size limit, and 8, without overflowing for any limit up to `usize::MAX`. Errors must give their offset in the input, and once the parser reports an error it must report the same error again. Errors and `Debug` output must never include names or values.

`form_pairs` must give, for input held in memory, the same pairs and error as the events.

## Rationale

The user asked for this parser in `base-web-url`, rather than in `base-web-content` as first agreed, because `base-web-content` depends on it, and it is the parser of form content and query strings alike. Mapping pairs onto application data, including nested structures and arrays, stays with `base-web-content`, which maps bracket paths onto declared structures, so the path is given as segments rather than the Ruby `protocol-url` gem's nested hash. The caller-owned buffer and events follow `base-web-multipart`'s `MultipartParser`, so that content of either kind streams through a consumer the same way.

The syntax, the skipped empty pairs, the distinction between a missing value and an empty one, the error for malformed percent-encoding, the limits on size, pair count, and depth with their defaults, and reading brackets after decoding follow the gem's `Protocol::URL::FormData::Parser`, `FormData::Nested`, and `Encoding.split`. The gem's form parser also decodes `+` as a space, as the WHATWG URL Standard does; its `Encoding.decode`, for queries, keeps `+`, but here a query has the form syntax, as `URLSearchParams` and HTML forms submitted with `GET` give it. The user requires strict decoding, since a consumer that sees replacement characters, or a `%` kept as text, can read a different value from another reader of the same input. The gem leaves invalid UTF-8 unchecked.

The bracket grammar is stricter than the gem's `Encoding.split`, which reads `[a]` as `a`, `a[b]c` as `a`, `b`, `c`, and skips an unmatched `[`, where other readers, such as Rack and PHP, read those names differently; a consumer mapping paths onto declared fields must not guess. Limits on each name and value were added to the gem's, so that the name the parser must hold whole is bounded and a consumer applies limits while the value streams. Lengths are counted as encoded, which bounds the bytes the parser holds and scans; decoded text is never longer.

## Scope

`FormParser`, `FormParsed`, `FormEvent`, `FormLimits`, `FormError`, `FormErrorKind`, `FormName`, `FormPath`, `FormPathSegment`, `FormPathSegments`, `FormPair`, `FormPairs`, and `form_pairs`.

## Validation

`tests/form_parser.rs` parses input split at every point and fed a byte at a time and compares it with the whole input parsed at once, for valid and invalid input, and covers each syntax rule, limit, error offset, and the redaction of `Debug` output. A property test compares generated input with a direct transcription of the WHATWG algorithm with strict decoding, whole and split at random points. `tests/form_path.rs` covers accepted and rejected bracket paths and the depth limit.
