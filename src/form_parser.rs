// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::syntax::hex_value;
use crate::{FormError, FormErrorKind, FormEvent, FormLimits, FormName, FormParsed};
use std::borrow::Cow;
use std::str::Utf8Error;

/// A sans-I/O parser for `application/x-www-form-urlencoded` content, and query strings, which use the same syntax, fed bytes as they arrive.
///
/// The caller owns the buffer. Each call to [`Self::parse`] reports how many bytes it consumed and what they held; the caller drops the consumed bytes, appends whatever arrives next, and calls again with what remains. Once the input has ended, the caller calls [`Self::parse_end`] with the bytes that remain, until it reports [`FormEvent::End`], since the last pair ends there. Each pair's complete decoded name is reported before its value, which arrives in decoded pieces, so the caller can refuse a name before reading its value, and never has to hold a whole value. The caller must be able to buffer [`Self::minimum_buffer_size`] bytes, to hold a whole name; the parser reports an error rather than needing more.
///
/// The syntax follows the WHATWG URL Standard: pairs are separated by `&`, and empty ones are skipped; a name ends at its first `=`, and a name without `=` has no value; `+` decodes to a space and `%XX` to its byte, in names and values. Decoding is strict: a `%` not followed by two hexadecimal digits, and decoded bytes that are not UTF-8, are errors.
///
/// ```
/// use base_web_url::{FormEvent, FormParser};
///
/// let mut parser = FormParser::new();
/// let (mut input, mut pairs) = (&b"name=Ada+Lovelace&email=ada%40example.com"[..], Vec::new());
/// loop {
///     let parsed = parser.parse_end(input)?;
///     match parsed.event {
///         FormEvent::Name(name) => pairs.push((name.as_str().to_owned(), String::new())),
///         FormEvent::Value(piece) => pairs.last_mut().unwrap().1.push_str(&piece),
///         FormEvent::PairEnd => {}
///         FormEvent::End => break,
///         FormEvent::Incomplete => unreachable!("parse_end has all of the remaining input"),
///     }
///     input = &input[parsed.consumed..];
/// }
/// assert_eq!(pairs, [("name".to_owned(), "Ada Lovelace".to_owned()), ("email".to_owned(), "ada@example.com".to_owned())]);
/// # Ok::<(), base_web_url::FormError>(())
/// ```
#[derive(Clone, Debug)]
pub struct FormParser {
    limits: FormLimits,
    state: State,
    /// The bytes consumed before the current call, which never exceeds the size limit.
    offset: usize,
    pair_count: usize,
    /// Where the input ends, once [`Self::parse_end`] has been called.
    end: Option<usize>,
    /// The error the parser reported, which it reports again for every later call.
    failed: Option<FormError>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Before a pair, where `&` separates empty pairs, which are skipped.
    Pair,
    /// In a pair's name, with the bytes of the input already searched for its end.
    Name { searched: usize },
    /// After a pair's name, in its value if it has one, with the bytes of the value consumed so far.
    Value { length: usize },
    /// The input has ended.
    End,
}

/// Why a step failed, with the offset of the problem from the start of the step's input.
type Failure = (FormErrorKind, usize);

/// The most bytes [`decode`] leaves for a later call at the end of an incomplete piece: an incomplete UTF-8 sequence of up to three bytes, each escaped, then a `%` and one hexadecimal digit.
const DECODING_HOLDBACK: usize = 3 * 3 + 2;

impl Default for FormParser {
    fn default() -> Self {
        Self::new()
    }
}

impl FormParser {
    /// A parser with the default limits.
    pub fn new() -> Self {
        Self::with_limits(FormLimits::default())
    }

    /// A parser with these limits.
    pub fn with_limits(limits: FormLimits) -> Self {
        Self {
            limits,
            state: State::Pair,
            offset: 0,
            pair_count: 0,
            end: None,
            failed: None,
        }
    }

    /// The limits this parser applies.
    pub fn limits(&self) -> FormLimits {
        self.limits
    }

    /// The input a caller must be able to buffer: given at least this many bytes, [`Self::parse`] always consumes some or reports an event other than [`FormEvent::Incomplete`]. It holds a whole name and the `=` or `&` after it, and the partial escape and UTF-8 sequence that may end a piece of a value.
    pub fn minimum_buffer_size(&self) -> usize {
        self.limits
            .name_length_limit
            .saturating_add(1)
            .max(DECODING_HOLDBACK + 1)
    }

    /// Whether [`FormEvent::End`] was reported.
    pub fn is_finished(&self) -> bool {
        self.state == State::End
    }

    /// Parse the next event from `input`, which must begin with the bytes the previous call did not consume, while more input may follow. Once [`Self::parse_end`] has been called, the input has ended, so this is the same as calling it.
    ///
    /// # Errors
    ///
    /// A [`FormError`] when the input is malformed or exceeds a limit. Once the parser reports an error, it reports the same error for every later call.
    pub fn parse<'a>(&mut self, input: &'a [u8]) -> Result<FormParsed<'a>, FormError> {
        self.run(input, false)
    }

    /// Parse the next event from `input`, which must be all of the input that remains after the bytes already consumed. Call it until it reports [`FormEvent::End`]; given all of the remaining input, it never reports [`FormEvent::Incomplete`]. The first call fixes where the input ends: later calls never read beyond it, and once the parser reports `End`, every later call reports `End` again.
    ///
    /// # Errors
    ///
    /// As for [`Self::parse`], and also when the input ends within a partial escape or UTF-8 sequence.
    pub fn parse_end<'a>(&mut self, input: &'a [u8]) -> Result<FormParsed<'a>, FormError> {
        self.run(input, true)
    }

    fn run<'a>(&mut self, input: &'a [u8], end: bool) -> Result<FormParsed<'a>, FormError> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        if end && self.end.is_none() {
            self.end = Some(self.offset.saturating_add(input.len()));
        }
        // Bytes beyond the end of the input, given to a later call, are never read.
        let input = match self.end {
            Some(end) => &input[..input.len().min(end - self.offset)],
            None => input,
        };
        let end = self.end.is_some_and(|end| end - self.offset == input.len());
        // Bytes beyond the size limit are never consumed, so a step that needs one fails; until then, the parser reports what the bytes within the limit hold, however the input is split.
        let remaining = self.limits.size_limit - self.offset;
        let truncated = input.len() > remaining;
        let window = &input[..input.len().min(remaining)];
        let mut consumed = 0;
        let result = match self.step(window, end && !truncated, &mut consumed) {
            Ok(FormEvent::Incomplete) if truncated => self
                .check_truncated(&window[consumed..], consumed)
                .and(Err((FormErrorKind::SizeLimit, remaining))),
            result => result,
        };
        match result {
            Ok(event) => {
                self.offset += consumed;
                Ok(FormParsed { consumed, event })
            }
            Err((kind, offset)) => {
                let error = FormError::new(kind, (self.offset + offset) as u64);
                self.failed = Some(error);
                Err(error)
            }
        }
    }

    /// Errors are reported in the order of the input, so when the size limit cuts a name short, a malformed name within the limit fails as one. A value's pieces were already decoded as they arrived.
    fn check_truncated(&self, rest: &[u8], consumed: usize) -> Result<(), Failure> {
        if matches!(self.state, State::Name { .. }) {
            decode(rest, false).map_err(|(kind, index)| (kind, consumed + index))?;
        }
        Ok(())
    }

    fn step<'a>(
        &mut self,
        input: &'a [u8],
        end: bool,
        consumed: &mut usize,
    ) -> Result<FormEvent<'a>, Failure> {
        loop {
            let rest = &input[*consumed..];
            match self.state {
                State::Pair => {
                    let separators = rest.iter().take_while(|&&byte| byte == b'&').count();
                    *consumed += separators;
                    if separators == rest.len() {
                        if end {
                            self.state = State::End;
                            return Ok(FormEvent::End);
                        }
                        return Ok(FormEvent::Incomplete);
                    }
                    // Each pair begins with a byte other than `&`, so the count cannot overflow.
                    self.pair_count += 1;
                    if self.pair_count > self.limits.pair_count_limit {
                        return Err((FormErrorKind::PairCountLimit, *consumed));
                    }
                    self.state = State::Name { searched: 0 };
                }
                State::Name { searched } => {
                    let limit = self.limits.name_length_limit;
                    // A name within the limit ends within its length and the `=` or `&` after it.
                    let scan = &rest[..rest.len().min(limit.saturating_add(1))];
                    let start = searched.min(scan.len());
                    let found = scan[start..]
                        .iter()
                        .position(|&byte| byte == b'=' || byte == b'&')
                        .map(|index| start + index);
                    let (length, has_value) = match found {
                        Some(index) => (index, scan[index] == b'='),
                        None if scan.len() > limit => {
                            // Errors are reported in the order of the input, so a malformed name within the limit fails as one.
                            decode(&rest[..limit], false)
                                .map_err(|(kind, index)| (kind, *consumed + index))?;
                            return Err((FormErrorKind::NameLengthLimit, *consumed + limit));
                        }
                        None if end => (rest.len(), false),
                        None => {
                            self.state = State::Name {
                                searched: scan.len(),
                            };
                            return Ok(FormEvent::Incomplete);
                        }
                    };
                    let (text, _) = decode(&rest[..length], true)
                        .map_err(|(kind, index)| (kind, *consumed + index))?;
                    let offset = (self.offset + *consumed) as u64;
                    *consumed += length + usize::from(has_value);
                    self.state = State::Value { length: 0 };
                    return Ok(FormEvent::Name(FormName::new(
                        text,
                        has_value,
                        offset,
                        self.limits.depth_limit,
                    )));
                }
                State::Value { length } => {
                    match rest.first() {
                        Some(b'&') => {
                            *consumed += 1;
                            self.state = State::Pair;
                            return Ok(FormEvent::PairEnd);
                        }
                        Some(_) => {}
                        None if end => {
                            self.state = State::Pair;
                            return Ok(FormEvent::PairEnd);
                        }
                        None => return Ok(FormEvent::Incomplete),
                    }
                    // SECURITY: A value is not held, only scanned for its end and decoded piece by piece, so its length limit defaults to the size limit; a consumer that knows what a value is for, such as `base-web-content`, applies a tighter one.
                    let allowance = self.limits.value_length_limit - length;
                    let scan = &rest[..rest.len().min(allowance.saturating_add(1))];
                    let (piece, complete) = match scan.iter().position(|&byte| byte == b'&') {
                        Some(index) => (&rest[..index], true),
                        None => {
                            let piece = &scan[..scan.len().min(allowance)];
                            (piece, end && piece.len() == rest.len())
                        }
                    };
                    let (text, used) = decode(piece, complete)
                        .map_err(|(kind, index)| (kind, *consumed + index))?;
                    if used == 0 {
                        // Only a partial escape or UTF-8 sequence is left within the allowance, so the value goes beyond it if anything but `&` follows.
                        if scan.len() > allowance {
                            return Err((FormErrorKind::ValueLengthLimit, *consumed + allowance));
                        }
                        return Ok(FormEvent::Incomplete);
                    }
                    *consumed += used;
                    self.state = State::Value {
                        length: length + used,
                    };
                    return Ok(FormEvent::Value(text));
                }
                State::End => return Ok(FormEvent::End),
            }
        }
    }
}

// SECURITY: Decoded names and values may contain any character, including NUL, CR, and LF from `%00` and `%0D%0A`, as the WHATWG URL Standard decodes them. This is not an injection weakness here: they are data, the parser interprets nothing in them but a bracket path's `[` and `]`, and a consumer that writes them into another syntax, such as a header or a log line, must encode them for it.
/// Decode `encoded`, a name or a piece of a value, giving its text, borrowed when nothing needed decoding, and how many bytes of `encoded` it covers. Unless `complete`, a partial escape or UTF-8 sequence at the end is left for a later call, which will have more of it. A failure gives the index of the problem in `encoded`.
fn decode(encoded: &[u8], complete: bool) -> Result<(Cow<'_, str>, usize), Failure> {
    let (mut index, mut escaped, mut invalid_escape) = (0, false, false);
    while index < encoded.len() {
        match encoded[index] {
            b'%' => {
                let digits = &encoded[index + 1..encoded.len().min(index + 3)];
                invalid_escape =
                    !digits.iter().all(u8::is_ascii_hexdigit) || (complete && digits.len() < 2);
                if digits.len() < 2 || invalid_escape {
                    break;
                }
                escaped = true;
                index += 3;
            }
            byte => {
                escaped |= byte == b'+';
                index += 1;
            }
        }
    }
    let decoded = decode_utf8(&encoded[..index], escaped, complete && !invalid_escape);
    if invalid_escape {
        // Errors are reported in the order of the input, however it is split, so invalid UTF-8 before the escape comes first.
        decoded?;
        return Err((FormErrorKind::PercentEncoding, index));
    }
    decoded
}

/// Check the decoded text of `encoded`, which has no partial or invalid escapes, and has an escape or `+` if `escaped`.
fn decode_utf8(
    encoded: &[u8],
    escaped: bool,
    complete: bool,
) -> Result<(Cow<'_, str>, usize), Failure> {
    let index = encoded.len();
    if !escaped {
        return match std::str::from_utf8(encoded) {
            Ok(text) => Ok((Cow::Borrowed(text), index)),
            Err(error) => {
                let valid = valid_length(encoded, error, complete)?;
                let text = std::str::from_utf8(&encoded[..valid]).expect("the prefix is UTF-8");
                Ok((Cow::Borrowed(text), valid))
            }
        };
    }
    match String::from_utf8(unescape(encoded)) {
        Ok(text) => Ok((Cow::Owned(text), index)),
        Err(error) => {
            let valid = valid_length(encoded, error.utf8_error(), complete)?;
            let mut bytes = error.into_bytes();
            bytes.truncate(valid);
            let text = String::from_utf8(bytes).expect("the prefix is UTF-8");
            Ok((Cow::Owned(text), encoded_index(encoded, valid)))
        }
    }
}

/// The length of the decoded bytes to keep, given that they are not all UTF-8: unless `complete`, an incomplete sequence at the end may be completed by a later call, but any other invalid sequence fails at its start in `encoded`.
fn valid_length(encoded: &[u8], error: Utf8Error, complete: bool) -> Result<usize, Failure> {
    if error.error_len().is_none() && !complete {
        Ok(error.valid_up_to())
    } else {
        Err((
            FormErrorKind::Utf8,
            encoded_index(encoded, error.valid_up_to()),
        ))
    }
}

/// Decode the escapes, already checked, and `+`, of `encoded`.
fn unescape(encoded: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut index = 0;
    while index < encoded.len() {
        let byte = encoded[index];
        if byte == b'%' {
            bytes.push(hex_value(encoded[index + 1]) << 4 | hex_value(encoded[index + 2]));
            index += 3;
        } else {
            bytes.push(if byte == b'+' { b' ' } else { byte });
            index += 1;
        }
    }
    bytes
}

/// The index in `encoded`, whose escapes are checked, of the decoded byte at `decoded`.
fn encoded_index(encoded: &[u8], decoded: usize) -> usize {
    (0..decoded).fold(0, |index, _| {
        index + if encoded[index] == b'%' { 3 } else { 1 }
    })
}
