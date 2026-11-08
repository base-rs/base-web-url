// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::FormName;
use std::borrow::Cow;
use std::fmt;

/// What one call to [`FormParser::parse`](crate::FormParser::parse) or [`FormParser::parse_end`](crate::FormParser::parse_end) found, with the number of input bytes it consumed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FormParsed<'a> {
    /// The input bytes consumed. The next call's input must begin with the bytes after them.
    pub consumed: usize,
    /// What the consumed bytes held.
    pub event: FormEvent<'a>,
}

/// One step through form content or a query string, borrowed from the parser's input when nothing needed decoding.
///
/// Input produces, for each pair, `Name`, any number of `Value`, then `PairEnd`, and finally `End`.
#[derive(Clone, PartialEq, Eq)]
pub enum FormEvent<'a> {
    /// A pair begins, with its complete decoded name.
    Name(FormName<'a>),
    /// Some of the current pair's decoded value, never empty. A value may arrive in any number of pieces, and a name without a value, or with an empty one, has none.
    Value(Cow<'a, str>),
    /// The current pair is complete.
    PairEnd,
    /// The input has ended, after the last pair, as [`FormParser::parse_end`](crate::FormParser::parse_end) reports. Once it is reported, every later call reports it again.
    End,
    /// The input ends before the next event: call again with more input, beginning with the bytes not yet consumed, or call [`FormParser::parse_end`](crate::FormParser::parse_end) if the input has ended.
    Incomplete,
}

/// Shows the length of a value rather than its text, since a value such as a password must not reach logs.
impl fmt::Debug for FormEvent<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => formatter.debug_tuple("Name").field(name).finish(),
            Self::Value(value) => write!(formatter, "Value(<{} bytes>)", value.len()),
            Self::PairEnd => formatter.write_str("PairEnd"),
            Self::End => formatter.write_str("End"),
            Self::Incomplete => formatter.write_str("Incomplete"),
        }
    }
}
