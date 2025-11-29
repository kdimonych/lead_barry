use core::iter::Iterator;
use core::option::Option::{self, None, Some};

pub trait StringSlicer {
    fn slice_by_lines(&self, max_len: usize) -> impl Iterator<Item = &'_ str>;
}

impl StringSlicer for str {
    fn slice_by_lines(&self, max_len: usize) -> impl Iterator<Item = &'_ str> {
        FitLineSlicer::new(self, max_len)
    }
}

// Word combinator
struct FitLineSlicer<'a> {
    message: &'a str,
    max_len: usize,
}

impl<'a> FitLineSlicer<'a> {
    fn new(message: &'a str, max_len: usize) -> Self {
        Self { message, max_len }
    }
}

impl<'a> Iterator for FitLineSlicer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.message.is_empty() {
            return None;
        }

        // Trim leading and trailing whitespace
        self.message = self.message.trim();
        let mut line = self.message;
        line = line.lines().next()?;

        let line_len = line.len_in_chars();
        if line_len <= self.max_len {
            // Fits entirely, return as is
            self.message = &self.message[line.len()..];
            return Some(line);
        }

        let extra = line_len - self.max_len;
        let mut it = line
            .char_indices() // Iterate over char indices
            .rev() // Reverse iterator
            .skip(extra - 1) // Skip extra chars - 1
            .peekable();

        let mut end = *it.peek()?;

        if !end.1.is_whitespace() {
            // If end position is inside the word, move back to the previous whitespace
            if let Some(ws) = it.find(|(_, c)| c.is_whitespace()) {
                end = ws;
            }
        }

        // Trim trailing whitespace if any
        line = line[..end.0].trim();
        self.message = self.message[line.len()..].trim();

        Some(line)
    }
}

pub trait StringTools {
    /// Returns the length of the string in characters (not bytes)
    /// # Example
    /// ```rust
    /// let s = "Hello, 世界!";
    /// assert_eq!(s.len_in_chars(), 9);
    /// ```
    fn len_in_chars(&self) -> usize;
}

impl StringTools for str {
    fn len_in_chars(&self) -> usize {
        self.chars().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Force std for tests even if building for embedded target
    #[cfg(test)]
    extern crate std;
    #[cfg(test)]
    use std::vec::Vec;

    #[test]
    fn test_len_in_chars() {
        let s = "Hello, 世界!";
        assert_eq!(s.len_in_chars(), 10); // Fixed: UTF-8 byte count vs char count
    }

    #[test]
    fn test_slice_by_lines() {
        let s = "This is a test message that will be sliced into multiple lines based on the maximum length specified.";
        let slicer = s.slice_by_lines(20);
        let lines: Vec<&str> = slicer.collect();
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0], "This is a test");
        assert_eq!(lines[1], "message that will");
        assert_eq!(lines[2], "be sliced into");
        assert_eq!(lines[3], "multiple lines");
        assert_eq!(lines[4], "based on the maximum length specified.");
    }
}
