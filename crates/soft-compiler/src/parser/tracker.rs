//! This module defines a [Tracker] structure that is used to keep track of
use std::{iter::Peekable, ops::Range, str::Chars};

use crate::location::Loc;

/// Keeps track of the current position in the source code and helps cutting out substrings. It's
/// used for parsing.

pub struct Tracker<'a> {
    current: Loc,
    saved: Vec<Loc>,
    string: &'a str,
    peekable: Peekable<Chars<'a>>,
}

impl<'a> Tracker<'a> {
    pub fn new(string: &'a str) -> Self {
        Self {
            current: Loc(0),
            saved: Vec::with_capacity(12),
            string,
            peekable: string.chars().peekable(),
        }
    }

    /// Returns the current last saved position in the source code.
    pub fn pop_range(&mut self) -> Range<Loc> {
        Range {
            start: self.saved.pop().unwrap(),
            end: self.current,
        }
    }

    /// Peeks the next character in the iterator.
    pub fn peek(&mut self) -> Option<char> {
        self.peekable.peek().cloned()
    }

    /// Gets the next character.
    pub fn next(&mut self) -> Option<char> {
        let next = self.peekable.next();
        if let Some(c) = next {
            self.current += Loc(c.len_utf8());
        }
        next
    }

    /// Jumps to the next character without returning a char.
    pub fn jump(&mut self) {
        self.next();
    }

    /// Saves the current position and then jumps one character.
    pub fn save_jump(&mut self) {
        self.save();
        self.next();
    }

    /// Gets the substring of the current code by a location.
    pub fn substring(&self, range: Range<Loc>) -> &'a str {
        &self.string[range.start.0..range.end.0]
    }

    pub fn save(&mut self) {
        self.saved.push(self.current)
    }
}
