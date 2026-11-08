// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use crate::{FormError, FormEvent, FormLimits, FormName, FormParser};
use std::borrow::Cow;
use std::fmt;
use std::iter::FusedIterator;

/// One name and value of input held in memory, borrowed from it when nothing needed decoding.
#[derive(Clone, PartialEq, Eq)]
pub struct FormPair<'a> {
    name: FormName<'a>,
    value: Option<Cow<'a, str>>,
}

impl<'a> FormPair<'a> {
    /// The decoded name.
    pub fn name(&self) -> &FormName<'a> {
        &self.name
    }

    /// The decoded value, which is `None` for a name without `=`, as in `a&b=1`, and empty for `a=`.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// The name and value.
    pub fn into_parts(self) -> (FormName<'a>, Option<Cow<'a, str>>) {
        (self.name, self.value)
    }
}

/// Shows the length of the value rather than its text, since a value such as a password must not reach logs.
impl fmt::Debug for FormPair<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormPair")
            .field("name", &self.name)
            .field(
                "value",
                &self.value.as_ref().map(|value| Length(value.len())),
            )
            .finish()
    }
}

/// The length of a value, shown in its place.
struct Length(usize);

impl fmt::Debug for Length {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "<{} bytes>", self.0)
    }
}

/// The pairs of form content or a query string held in memory, in order, as [`form_pairs`] produces them.
#[derive(Clone)]
pub struct FormPairs<'a> {
    parser: FormParser,
    input: &'a [u8],
    position: usize,
    failed: bool,
}

/// The pairs of form content or a query string held in memory, with the default limits.
///
/// ```
/// use base_web_url::{Url, form_pairs};
///
/// let url = Url::parse("https://example.com/search?q=fish+%26+chips&page=2&debug")?;
/// let mut pairs = Vec::new();
/// for pair in form_pairs(url.query().unwrap_or("")) {
///     let pair = pair?;
///     pairs.push((pair.name().as_str().to_owned(), pair.value().map(str::to_owned)));
/// }
/// assert_eq!(pairs, [
///     ("q".to_owned(), Some("fish & chips".to_owned())),
///     ("page".to_owned(), Some("2".to_owned())),
///     ("debug".to_owned(), None),
/// ]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn form_pairs<B: AsRef<[u8]> + ?Sized>(input: &B) -> FormPairs<'_> {
    FormPairs::with_limits(input, FormLimits::default())
}

impl<'a> FormPairs<'a> {
    /// The pairs of input held in memory, with these limits.
    pub fn with_limits<B: AsRef<[u8]> + ?Sized>(input: &'a B, limits: FormLimits) -> Self {
        Self {
            parser: FormParser::with_limits(limits),
            input: input.as_ref(),
            position: 0,
            failed: false,
        }
    }

    fn next_pair(&mut self) -> Result<Option<FormPair<'a>>, FormError> {
        let input = self.input;
        let mut pair = None;
        loop {
            let parsed = self.parser.parse_end(&input[self.position..])?;
            self.position += parsed.consumed;
            match parsed.event {
                FormEvent::Name(name) => {
                    let value = name.has_value().then_some(Cow::Borrowed(""));
                    pair = Some(FormPair { name, value });
                }
                // Given all of the remaining input, the parser reports a value in one piece, or reports an error after its first piece, when it reaches a limit.
                FormEvent::Value(piece) => match pair.as_mut().and_then(|pair| pair.value.as_mut())
                {
                    Some(value) if value.is_empty() => *value = piece,
                    _ => unreachable!("a value arrives in one piece after a name with `=`"),
                },
                FormEvent::PairEnd => return Ok(pair),
                FormEvent::End => return Ok(None),
                FormEvent::Incomplete => {
                    unreachable!("parse_end has all of the remaining input")
                }
            }
        }
    }
}

/// Shows the position in the input and its length, since the input must not reach logs.
impl fmt::Debug for FormPairs<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FormPairs")
            .field("position", &self.position)
            .field("length", &self.input.len())
            .finish()
    }
}

impl<'a> Iterator for FormPairs<'a> {
    type Item = Result<FormPair<'a>, FormError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        let result = self.next_pair().transpose();
        self.failed = !matches!(result, Some(Ok(_)));
        result
    }
}

impl FusedIterator for FormPairs<'_> {}
