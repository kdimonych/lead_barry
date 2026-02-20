use core::iter::Iterator;

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
    line_splitter: core::str::Lines<'a>,
    active_line: &'a str,
    max_len: usize,
}

impl<'a> FitLineSlicer<'a> {
    fn new(message: &'a str, max_len: usize) -> Self {
        let res = Self {
            line_splitter: message.lines(),
            active_line: &message[..0],
            max_len,
        };
        res
    }
}

impl<'a> Iterator for FitLineSlicer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.active_line.is_empty() {
            self.active_line = self.line_splitter.next()?;
        }

        // Trim leading and trailing whitespace
        self.active_line = self.active_line.trim();
        let mut line = self.active_line;

        let line_len = line.len_in_chars();
        if line_len <= self.max_len {
            // Fits entirely, return as is
            line = line.trim();
            self.active_line = &self.active_line[line.len()..].trim();
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
        self.active_line = self.active_line[line.len()..].trim();

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
        let s = "This is a test message\n that will be sliced into multiple \nlines based on the maximum length specified.";
        let slicer = s.slice_by_lines(20);
        let lines: Vec<&str> = slicer.collect();
        assert_eq!(lines.len(), 7);
        assert_eq!(lines[0], "This is a test");
        assert_eq!(lines[1], "message");
        assert_eq!(lines[2], "that will be sliced");
        assert_eq!(lines[3], "into multiple");
        assert_eq!(lines[4], "lines based on the");
        assert_eq!(lines[5], "maximum length");
        assert_eq!(lines[6], "specified.");
    }

    #[test]
    fn test_slice_by_lines_nl_chain() {
        let s = "This is a test message\n that will be sliced into multiple \n\n\nlines based on the maximum length specified.";
        let slicer = s.slice_by_lines(20);
        let lines: Vec<&str> = slicer.collect();
        assert_eq!(lines.len(), 9);
        assert_eq!(lines[0], "This is a test");
        assert_eq!(lines[1], "message");
        assert_eq!(lines[2], "that will be sliced");
        assert_eq!(lines[3], "into multiple");
        assert_eq!(lines[4], "");
        assert_eq!(lines[5], "");
        assert_eq!(lines[6], "lines based on the");
        assert_eq!(lines[7], "maximum length");
        assert_eq!(lines[8], "specified.");
    }

    #[test]
    fn test_slice_by_lines_n_th_char_followed_by_nl_must_not_generate_extra_nl() {
        let s = "This is a test message\n that will be sliced into multiple \n\n\nlines based on the maximum length specified.";
        let slicer = s.slice_by_lines(22);
        let lines: Vec<&str> = slicer.collect();
        assert_eq!(lines.len(), 8);
        assert_eq!(lines[0], "This is a test message"); // No extra split here, as max_len is 22
        // and \n marker is at position 23 that requires to split before it by consumind the marker.
        assert_eq!(lines[1], "that will be sliced");
        assert_eq!(lines[2], "into multiple");
        assert_eq!(lines[3], "");
        assert_eq!(lines[4], "");
        assert_eq!(lines[5], "lines based on the");
        assert_eq!(lines[6], "maximum length");
        assert_eq!(lines[7], "specified.");
    }
}
