/// A read-only string container with constrained capacity that can either be a static str or a heapless::String with a fixed capacity.
/// This is useful for scenarios where you want to avoid dynamic memory allocation but still need some flexibility in string handling
/// especially for send use cases.
/// The size of the string is fixed at compile time, and the implementation ensures that the string
/// does not exceed this size, either by truncating or by panicking on creation if the input is too large.
/// This type guarantees that the contained string is always valid UTF-8.
///# Examples
/// ```
/// let s1: AnyString<10> = AnyString::from_str("Hello");
/// assert_eq!(s1.as_str(), "Hello");
/// let s2: AnyString<10> = AnyString::from_heapless(heapless::String::from("World"));
/// assert_eq!(s2.as_str(), "World");
/// let s3: AnyString<5> = AnyString::from_str_truncate("Hello, World!");
/// assert_eq!(s3.as_str(), "Hello");
/// ```
/// # Panics
/// The `from_str` method will panic if the input string exceeds the fixed size.
/// The `from_heapless` method will panic if the input heapless::String exceeds the fixed size.
/// The `from_str_truncate` method will not panic, but will truncate the input string if it exceeds the fixed size.
/// # Notes
/// This implementation uses the `heapless` crate for the heapless::String type.
/// Ensure that the `heapless` crate is included in your dependencies.
pub struct AnyString<'a, const SIZE: usize> {
    inner: AnyStringInner<'a, SIZE>,
}

enum AnyStringInner<'a, const SIZE: usize> {
    Static(&'a str),
    Heapless(heapless::String<SIZE>),
}

impl<'a, const SIZE: usize> From<heapless::String<SIZE>> for AnyString<'a, SIZE> {
    /// Creates an `AnyString` from a `heapless::String` with the same capacity.
    fn from(s: heapless::String<SIZE>) -> Self {
        AnyString::from_heapless(s)
    }
}

impl<'a, const SIZE: usize> AnyString<'a, SIZE> {
    /// Creates an `AnyString` from a static string slice.
    /// Panics if the length of the provided string exceeds the fixed size. This is a compile-time check.
    pub const fn from_str(s: &'a str) -> Self {
        if s.len() > SIZE {
            panic!("String length exceeds the fixed size");
        }

        AnyString {
            inner: AnyStringInner::Static(s),
        }
    }

    /// Creates an `AnyString` from a `heapless::String` with the same capacity.
    /// Panics if the length of the provided string exceeds the fixed size.
    /// This is a runtime check because `heapless::String` does not enforce
    /// its length at compile time.
    pub fn from_heapless(s: heapless::String<SIZE>) -> Self {
        AnyString {
            inner: AnyStringInner::Heapless(s),
        }
    }

    /// Creates an `AnyString` from a string slice, truncating it if necessary to fit the fixed size.
    /// This is a runtime operation.
    /// If the input string is longer than the fixed size, it will be cut off at the size limit.
    /// If it is shorter, it will be used as-is.
    /// The resulting `AnyString` will always be valid UTF-8.
    /// # Examples
    /// ```
    /// let s: AnyString<5> = AnyString::from_str_truncate("Hello, World!");
    /// assert_eq!(s.as_str(), "Hello");
    /// let s: AnyString<20> = AnyString::from_str_truncate("Hi");
    /// assert_eq!(s.as_str(), "Hi");
    /// ```
    pub fn from_str_truncate(s: &str) -> Self {
        let mut heapless_str = heapless::String::<SIZE>::new();
        let _ = heapless_str.push_str(&s[..core::cmp::min(s.len(), SIZE)]);
        AnyString::from_heapless(heapless_str)
    }

    /// Returns the string slice contained in this `AnyString`.
    /// The returned string slice is valid as long as this `AnyString` is valid.
    /// This is guaranteed to be a valid UTF-8 string slice.
    pub fn as_str(&self) -> &str {
        match &self.inner {
            AnyStringInner::Static(s) => s,
            AnyStringInner::Heapless(s) => s.as_str(),
        }
    }

    /// Returns the length of this `AnyString`, in bytes.
    /// This is the number of bytes that are currently used in the string,
    /// not the capacity of the string.
    pub fn len(&self) -> usize {
        match &self.inner {
            AnyStringInner::Static(s) => s.len(),
            AnyStringInner::Heapless(s) => s.len(),
        }
    }

    /// Returns true if this `AnyString` is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the capacity of this `AnyString`, which is the maximum number of bytes it can hold.
    pub const fn capacity() -> usize {
        SIZE
    }

    /// Returns an empty `heapless::String` with the same capacity as this `AnyString`.
    /// This can be useful for creating a new `heapless::String` to populate and
    /// then convert into an `AnyString`.
    pub const fn complimentary_str() -> heapless::String<SIZE> {
        heapless::String::<SIZE>::new()
    }
}

// TODO: Implement macro for easier creation of AnyString instances with compile-time checks.
