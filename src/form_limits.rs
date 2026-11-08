// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

/// The limits a [`FormParser`](crate::FormParser) applies, so that hostile input cannot make it buffer, scan, or count without bound.
///
/// Lengths and sizes count bytes as they are encoded in the input, before decoding; decoded text is never longer.
///
/// ```
/// use base_web_url::FormLimits;
///
/// let limits = FormLimits { pair_count_limit: 16, ..FormLimits::default() };
/// assert_eq!(limits.size_limit, FormLimits::SIZE_LIMIT);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FormLimits {
    /// The bytes of the whole input.
    pub size_limit: usize,
    /// The pairs in the input. Empty pairs, as in `a=1&&b=2`, are skipped and not counted.
    pub pair_count_limit: usize,
    /// The bytes of one pair's name, which the parser must hold whole, so [`FormParser::minimum_buffer_size`](crate::FormParser::minimum_buffer_size) depends on it.
    pub name_length_limit: usize,
    /// The bytes of one pair's value, not counting the `=` before it.
    pub value_length_limit: usize,
    /// The segments of a bracket path, the first included, which [`FormName::path`](crate::FormName::path) reads, so `a[b][]` has a depth of 3.
    pub depth_limit: usize,
}

impl FormLimits {
    /// The default size limit: 2 MiB.
    pub const SIZE_LIMIT: usize = 2 * 1024 * 1024;
    /// The default pair count limit.
    pub const PAIR_COUNT_LIMIT: usize = 1024;
    /// The default name length limit: 1 KiB.
    pub const NAME_LENGTH_LIMIT: usize = 1024;
    /// The default value length limit, the default size limit, so that by default only the size of the input limits a value.
    pub const VALUE_LENGTH_LIMIT: usize = Self::SIZE_LIMIT;
    /// The default depth limit.
    pub const DEPTH_LIMIT: usize = 8;
}

impl Default for FormLimits {
    fn default() -> Self {
        Self {
            size_limit: Self::SIZE_LIMIT,
            pair_count_limit: Self::PAIR_COUNT_LIMIT,
            name_length_limit: Self::NAME_LENGTH_LIMIT,
            value_length_limit: Self::VALUE_LENGTH_LIMIT,
            depth_limit: Self::DEPTH_LIMIT,
        }
    }
}
