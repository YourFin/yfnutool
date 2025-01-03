use crate::Id;
use anyhow::{anyhow, Context, Result};
use bstr::{BStr, BString, ByteSlice};
#[cfg(test)]
use pretty_assertions::{assert_eq, assert_ne};
use std::cmp;
use std::cmp::Ordering;
use std::fmt;
use std::iter;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

pub trait StrRepr<Src: std::fmt::Display + ?Sized>: ToStrRepr {
    fn str_repr(lit: Src) -> Self;
}
pub fn str_repr<Src: std::fmt::Display + Sized, Cli: StrRepr<Src>>(lit: Src) -> Cli {
    StrRepr::str_repr(lit)
}
pub trait ToStrRepr {
    fn to_str_repr(&self) -> String;
}
impl<T: ToStrRepr> fmt::Debug for Id<&T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str_repr = self.0.to_str_repr();
        fmt.debug_tuple(std::any::type_name::<T>())
            .field(&str_repr)
            .finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct Bytes {
    pub text: BString,
    pub cursor_pos: usize,
}
impl<T: AsRef<[u8]> + fmt::Display + Sized> StrRepr<T> for Bytes {
    fn str_repr(lit: T) -> Self {
        let bytes = lit.as_ref();
        for (idx, char) in bytes.iter().enumerate() {
            if *char == b'|' {
                let mut ret = Vec::with_capacity(bytes.len() - 1);
                ret.extend_from_slice(&bytes[0..idx]);
                ret.extend_from_slice(&bytes[idx + 1..bytes.len()]);
                return Bytes {
                    text: BString::new(ret),
                    cursor_pos: idx,
                };
            }
        }
        panic!("No '|' in `{}`", &lit);
    }
}

impl ToStrRepr for Bytes {
    fn to_str_repr(&self) -> String {
        let mut ret = Vec::with_capacity(self.text.len() + 1);
        for idx in 0..(self.text.len() + 1) {
            ret.push(match idx.cmp(&self.cursor_pos) {
                Ordering::Less => self.text[idx],
                Ordering::Equal => b'|',
                Ordering::Greater => self.text[idx - 1],
            })
        }
        String::from_utf8_lossy(&ret).into_owned()
    }
}
impl fmt::Debug for Bytes {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Id(self).fmt(fmt)
    }
}

impl Bytes {
    pub fn insert_push_cursor(&mut self, pos: usize, char: u8) {
        self.insert_wrt_cursor(pos, char, true);
    }
    pub fn insert_no_push_cursor(&mut self, pos: usize, char: u8) {
        self.insert_wrt_cursor(pos, char, false);
    }
    fn insert_wrt_cursor(&mut self, pos: usize, char: u8, push_cursor: bool) {
        if pos == self.text.len() {
            self.text.push(char)
        } else {
            self.text.insert(pos, char);
        }
        let less_than_for_cursor = if push_cursor {
            cmp::PartialOrd::le
        } else {
            cmp::PartialOrd::lt
        };
        if less_than_for_cursor(&pos, &self.cursor_pos) {
            self.cursor_pos += 1;
        }
    }
    fn delete_pull_cursor(&mut self, pos: usize) {
        self.delete_wrt_cursor(pos, true);
    }
    fn delete_no_pull_cursor(&mut self, pos: usize) {
        self.delete_wrt_cursor(pos, false);
    }
    fn delete_wrt_cursor(&mut self, pos: usize, pull_cursor: bool) {
        if pos == self.text.len() {
            self.text.pop();
        } else {
            self.text.remove(pos);
        }
        let less_than_for_cursor = if pull_cursor {
            cmp::PartialOrd::le
        } else {
            cmp::PartialOrd::lt
        };
        if less_than_for_cursor(&pos, &self.cursor_pos) {
            self.cursor_pos -= 1;
        }
    }
    // If the cursor is:
    //   After the characters to be replaced: (>= chars_to_replace.end) it gets pushed by the length of the replacement
    //   Otherwise: unmoved
    pub fn overwrite_range(&mut self, chars_to_replace: Range<usize>, replacement: &[u8]) {
        if chars_to_replace == (self.text.len()..self.text.len()) {
            self.text.extend_from_slice(replacement);
        } else {
            self.text =
                Self::overwrite_range_iter(&self.text, &chars_to_replace, replacement).collect();
        }
        if self.cursor_pos >= chars_to_replace.end {
            self.cursor_pos = self.cursor_pos - chars_to_replace.len() + replacement.len();
        }
    }

    fn overwrite_range_iter<'a>(
        source: &'a [u8],
        replacement_range: &'a Range<usize>,
        replacement: &'a [u8],
    ) -> impl Iterator<Item = u8> + use<'a> {
        source
            .iter()
            .enumerate()
            .flat_map(|(idx, char)| {
                if idx == replacement_range.start {
                    replacement
                        .iter()
                        .map(|cc| (either::Left(()), *cc))
                        .chain(iter::once((either::Right(idx), *char)))
                        .collect::<Vec<_>>()
                } else {
                    iter::once((either::Right(idx), *char)).collect()
                }
            })
            .flat_map(|val: (either::Either<(), usize>, u8)| match val {
                (either::Right(idx), _) if replacement_range.contains(&idx) => None,
                (_, char) => Some(char),
            })
    }
}

#[cfg(test)]
mod impl_tests {
    use super::str_repr;
    use super::*;
    use yare::parameterized;
    mod early_move_cursor {
        use super::*;
        #[parameterized(
                before_cursor = {str_repr(" |  "), 0, b'i', str_repr("i |  ")},
                at_cursor     = {str_repr(" |  "), 1, b'i', str_repr(" i|  ")},
                after_cursor  = {str_repr(" |  "), 2, b'i', str_repr(" | i ")},
                at_end        = {str_repr(" |  "), 3, b'i', str_repr(" |  i")},
            )]
        fn should_correctly_adjust_cursor_pos_inserting_character(
            before: Bytes,
            pos: usize,
            char: u8,
            expected: Bytes,
        ) {
            let mut actual = before.clone();
            actual.insert_push_cursor(pos, char);
            pretty_assertions::assert_eq!(actual, expected);
        }
        #[parameterized(
                before_cursor = {str_repr("d|  "), 0, str_repr("|  ")},
                at_cursor     = {str_repr(" |d "), 1, str_repr("|  ")},
                after_cursor  = {str_repr(" | d"), 2, str_repr(" | ")},
            )]
        fn should_correctly_adjust_cursor_pos_deleting_character(
            before: Bytes,
            pos: usize,
            expected: Bytes,
        ) {
            let mut actual = before.clone();
            actual.delete_pull_cursor(pos);
            pretty_assertions::assert_eq!(actual, expected);
        }
    }
    mod no_early_move_cursor {
        use super::*;
        #[parameterized(
                before_cursor = {str_repr(" |  "), 0, b'i', str_repr("i |  ")},
                at_cursor     = {str_repr(" |  "), 1, b'i', str_repr(" |i  ")},
                after_cursor  = {str_repr(" |  "), 2, b'i', str_repr(" | i ")},
                at_end        = {str_repr(" |  "), 3, b'i', str_repr(" |  i")},
            )]
        fn should_correctly_adjust_cursor_pos_inserting_character(
            before: Bytes,
            pos: usize,
            char: u8,
            expected: Bytes,
        ) {
            let mut actual = before.clone();
            actual.insert_no_push_cursor(pos, char);
            pretty_assertions::assert_eq!(actual, expected);
        }
        #[parameterized(
                before_cursor = {str_repr("d|  "), 0, str_repr("|  ")},
                at_cursor     = {str_repr(" |d "), 1, str_repr(" | ")},
                after_cursor  = {str_repr(" | d"), 2, str_repr(" | ")},
            )]
        fn should_correctly_adjust_cursor_pos_deleting_character(
            before: Bytes,
            pos: usize,
            expected: Bytes,
        ) {
            let mut actual = before.clone();
            actual.delete_no_pull_cursor(pos);
            pretty_assertions::assert_eq!(actual, expected);
        }
    }

    mod overwrite_range {
        use super::*;
        #[parameterized(
                at_start  = {str_repr("0|123"), 0..0, b"i", str_repr("i0|123")},
                before_replace_with_less  =
                  {str_repr("01234|5"), 2..3, b"i", str_repr("01i34|5")},
                before_replace_with_more  =
                  {str_repr("01234|5"), 2..3, b"iii", str_repr("01iii34|5")},
                before_replace_with_same  = {str_repr("01234|5"), 2..3, b"i", str_repr("01i34|5")},
                at_end  = {str_repr("0|123"), 4..4, b"i", str_repr("0|123i")},
                at_end_cursor_at_end  = {str_repr("0123|"), 4..4, b"i", str_repr("0123i|")},
            )]
        fn should_correctly_adjust_cursor_pos_inserting_character(
            before: Bytes,
            range: Range<usize>,
            replacement: &[u8],
            expected: Bytes,
        ) {
            let mut actual = before.clone();
            actual.overwrite_range(range, replacement);
            pretty_assertions::assert_eq!(actual, expected);
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Utf8 {
    pub text: String,
    pub cursor_pos_grapheme: usize,
}

impl TryFrom<Bytes> for Utf8 {
    type Error = anyhow::Error;
    fn try_from(bytes: Bytes) -> Result<Self> {
        if bytes.cursor_pos == bytes.text.len() {
            return Ok(Utf8 {
                cursor_pos_grapheme: bytes.text.graphemes().count(),
                text: bytes
                    .text
                    .try_into()
                    .with_context(|| "When converting to Utf8 Command line")?,
            });
        }
        let pos_result = bytes
            .text
            .grapheme_indices()
            .enumerate()
            .find(|(_idx, (grapheme_start_byte, _, _))| *grapheme_start_byte == bytes.cursor_pos)
            .map(|(cursor_pos_grapheme_idx, _)| cursor_pos_grapheme_idx);
        let text = bytes
            .text
            .try_into()
            .with_context(|| "When converting to Utf8 Command line")?;
        // Only surface cursor position errors on valid utf8
        let cursor_pos_grapheme = pos_result.ok_or_else(|| {
            anyhow!(
                "Byte index of cursor {} does not fall on a grapheme boundary in {:?}",
                bytes.cursor_pos,
                text
            )
        })?;
        Ok(Utf8 {
            text,
            cursor_pos_grapheme,
        })
    }
}
impl From<Utf8> for Bytes {
    fn from(value: Utf8) -> Self {
        let cursor_pos = match value
            .text
            .grapheme_indices(true)
            .nth(value.cursor_pos_grapheme)
        {
            Some((grapheme_start_byte, _)) => grapheme_start_byte,
            // Assumes that the cursor is at the end
            // This could silently be wrong for an ill-constructed Utf8
            None => value.text.len(),
        };

        Bytes {
            cursor_pos,
            text: value.text.into(),
        }
    }
}

impl ToStrRepr for Utf8 {
    fn to_str_repr(&self) -> String {
        let mut result = String::with_capacity(self.text.len() + 1);
        let mut pushed_cursor = false;
        for (idx, grapheme) in self.text.graphemes(true).enumerate() {
            if idx == self.cursor_pos_grapheme {
                pushed_cursor = true;
                result.push('|');
            }
            result.push_str(grapheme);
        }
        if !pushed_cursor {
            result.push('|');
        }
        result
    }
}
impl<T: AsRef<str> + fmt::Display + Sized> StrRepr<T> for Utf8 {
    fn str_repr(lit: T) -> Self {
        let input = lit.as_ref();
        let mut result_text = String::with_capacity(input.len() - 1);
        let mut result_idx: Option<usize> = None;
        for (idx, grapheme) in input.graphemes(true).enumerate() {
            if grapheme == "|" {
                if result_idx == None {
                    result_idx = Some(idx);
                } else {
                    panic!("Multiple | characters in str_repr input");
                }
            } else {
                result_text.push_str(grapheme);
            }
        }
        Utf8 {
            cursor_pos_grapheme: result_idx.unwrap(),
            text: result_text,
        }
    }
}
impl fmt::Debug for Utf8 {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        Id(self).fmt(fmt)
    }
}

impl Utf8 {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    mod str_repr {
        use super::*;

        proptest! {
            #[test]
            fn roundtrip_bytes(
                (text, cursor_pos) in "[^|]*".prop_flat_map(|str| {
                    let bytes = str.as_bytes();
                    (Just(bytes.to_owned().into()), 0..(bytes.len() + 1))
                })
            ) {
                let original = Bytes { cursor_pos, text };
                let repr = original.to_str_repr();
                prop_assume!(repr.chars().find(|chr| { *chr == std::char::REPLACEMENT_CHARACTER }).is_none());
                prop_assert_eq!(original, str_repr(repr));
            }
        }
        proptest! {
            #[test]
            fn roundtrip_utf8(
                (text, cursor_pos_grapheme) in "[^|]*".prop_flat_map(|str| {
                    let graphemes = str.graphemes(true).count();
                    (Just(str.to_string()), 0..(graphemes+1))
                })
            ) {
                let original = Utf8 { cursor_pos_grapheme, text };
                let repr = original.to_str_repr();
                prop_assert_eq!(original, str_repr(repr));
            }
        }
    }
    proptest! {
        #[test]
        fn roundtrip_utf8_to_bytes(
            (text, cursor_pos_grapheme) in "[^|]*".prop_flat_map(|str| {
                let graphemes = str.graphemes(true).count();
                (Just(str.to_string()), 0..(graphemes+1))
            })
        ) {
            let original = Utf8 { cursor_pos_grapheme, text };
            prop_assert_eq!(original.clone(), <Utf8 as Into<Bytes>>::into(original).try_into().unwrap());
        }

    }
}
