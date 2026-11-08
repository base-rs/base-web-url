// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use std::fmt;

/// Why form content or a query string could not be parsed, with the offset in the input where the problem was found. It never includes names or values, which may be secret or untrusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormError {
    kind: FormErrorKind,
    offset: u64,
}

/// The kind of [`FormError`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FormErrorKind {
    /// The input is longer than its limit. The offset is the first byte beyond it.
    SizeLimit,
    /// The input has more pairs than its limit. The offset is where the first pair beyond it begins.
    PairCountLimit,
    /// A name is longer than its limit. The offset is the first byte beyond it.
    NameLengthLimit,
    /// A value is longer than its limit. The offset is the first byte beyond it.
    ValueLengthLimit,
    /// A `%` is not followed by two hexadecimal digits. The offset is the `%`.
    PercentEncoding,
    /// The decoded bytes of a name or value are not UTF-8. The offset is where the invalid sequence begins, or, for an escape, its `%`.
    Utf8,
    /// A name read as a bracket path is not `key`, followed by any number of `[key]` and `[]`, where a key is not empty and contains neither `[` nor `]`. The offset is the name's.
    InvalidPath,
    /// A name read as a bracket path has more segments than its limit. The offset is the name's.
    DepthLimit,
}

impl FormError {
    pub(crate) fn new(kind: FormErrorKind, offset: u64) -> Self {
        Self { kind, offset }
    }

    /// What went wrong.
    pub fn kind(&self) -> FormErrorKind {
        self.kind
    }

    /// The offset in the input, in bytes, where the problem was found.
    pub fn offset(&self) -> u64 {
        self.offset
    }
}

impl fmt::Display for FormError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let description = match self.kind {
            FormErrorKind::SizeLimit => "form content exceeds its size limit",
            FormErrorKind::PairCountLimit => "form content exceeds its pair count limit",
            FormErrorKind::NameLengthLimit => "form name exceeds its length limit",
            FormErrorKind::ValueLengthLimit => "form value exceeds its length limit",
            FormErrorKind::PercentEncoding => "invalid percent-encoding in form content",
            FormErrorKind::Utf8 => "form content is not UTF-8",
            FormErrorKind::InvalidPath => "form name is not a bracket path",
            FormErrorKind::DepthLimit => "form name exceeds its depth limit",
        };
        write!(formatter, "{description} at byte {}", self.offset)
    }
}

impl std::error::Error for FormError {}
