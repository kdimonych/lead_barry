pub enum AnyString<'a, const SIZE: usize> {
    Static(&'a str),
    Heapless(heapless::String<SIZE>),
}

impl<'a, const SIZE: usize> From<&'a str> for AnyString<'a, SIZE> {
    fn from(s: &'a str) -> Self {
        AnyString::Static(s)
    }
}

impl<'a, const SIZE: usize> From<heapless::String<SIZE>> for AnyString<'a, SIZE> {
    fn from(s: heapless::String<SIZE>) -> Self {
        AnyString::Heapless(s)
    }
}

impl<'a, const SIZE: usize> AnyString<'a, SIZE> {
    pub fn as_str(&self) -> &str {
        match self {
            AnyString::Static(s) => s,
            AnyString::Heapless(s) => s.as_str(),
        }
    }
}
