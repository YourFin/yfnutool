//use std::io::IsTerminal;

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, log_enabled};
mod nu_kind_sym;
use nu_kind_sym::nu_kind_sym;
#[cfg(test)]
use pretty_assertions::{assert_eq, assert_ne};
use std::cmp::Ordering;
use std::fmt;
use std::io::Write;
//use std::str;
//use tree_sitter::{InputEdit, Language, Point};

#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    text: String,
}

//#[derive(Debug, PartialEq, Eq)]
//enum ParseState {
//    SingleQuote,
//    DoubleQuote,
//    RawString,
//    // Ignoring bare-word strings
//    Backtick,
//    SingleQuoteInterpolated,
//    DoubleQuoteInterpolated,
//
//    // "not in the middle of a string"
//    Other,
//
//    DollarSign,
//    RawStringInR,
//    RawStringInPound,
//    RawStringInSingle,
//}

// h r#'
// ^ boring
//  ^ boring
//   ^ maybe an r#' ?
//    ^ maybe an r#' ?
//     ^ An r#'!

use cmd_line::CmdLine;

fn dwim_interpolate_cli(mut input: CmdLine) -> Result<CmdLine> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_nu::LANGUAGE.into())
        .with_context(|| "Error loading nu grammar")?;

    let tree = parser
        .parse(&input.text, None)
        .with_context(|| "Tree-sitter unable to parse tree from input")?;

    if input.text == b"" {
        return Ok(cmd_line::str_repr(r#"$"(|)""#));
    }

    // Handle the edge case where the cursor is off the end of the
    let cursor_at_end = input.cursor_pos == input.text.len();
    let effective_cursor_pos = if cursor_at_end {
        input.cursor_pos - 1
    } else {
        input.cursor_pos
    };

    let innermost_node = tree
        .root_node()
        .descendant_for_byte_range(effective_cursor_pos, effective_cursor_pos)
        .with_context(|| {
            format!(
                "Unable to find node at cursor position {}",
                effective_cursor_pos
            )
        })?;
    if log_enabled!(log::Level::Debug) {
        let mut node = innermost_node;
        // evil do while
        'l: while {
            debug!("node parent chain: {}", node.kind());
            let cont = node.id() != tree.root_node().id();
            let optional_node = node.parent();
            node = match optional_node {
                Some(wrapped) => wrapped,
                None => break 'l,
            };
            cont
        } {}
    }

    if innermost_node.kind_id() == nu_kind_sym!("val_string") {
        match input.text[innermost_node.start_byte()] {
            b'\'' => {
                debug!("Single quote string");
                input.insert_push_cursor(innermost_node.start_byte(), b'$');
                input.insert_push_cursor(input.cursor_pos, b'(');
                input.insert_no_push_cursor(input.cursor_pos, b')');
                return Ok(input);
            }
            _ => (),
        }
    }

    Ok(CmdLine {
        text: vec![],
        cursor_pos: 0,
    })
}

fn main() -> Result<()> {
    let cl = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cl.verbose.log_level_filter())
        .init();
    //info!("starting up");
    //warn!("weewooweewoo");
    //let is_terminal = std::io::stdout().is_terminal();
    //debug!("std::io::stdout().is_terminal: {}", is_terminal);
    //let path = "test.txt";
    //let content =
    //    std::fs::read_to_string(path).with_context(|| format!("could not read file `{}`", path))?;
    //println!("file content: {}", content);
    let result = dwim_interpolate_cli(cmd_line::str_repr(&cl.text))
        .with_context(|| format!("Error running against {}", cl.text))?;
    std::io::stdout().write_all(&cmd_line::to_str_repr(result))?;
    std::io::stdout().write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmd_line::str_repr;
    use yare::parameterized;

    #[parameterized(
        //in_existing_double_quote_string = {str_repr(r#"foo "ba| "#), str_repr(r#"foo $"ba(|)"#)},
        in_existing_single_quote_string = {str_repr(r#"foo 'ba| '"#), str_repr(r#"foo $'ba(|) '"#)},
        empty_string = {str_repr("|"), str_repr(r#"$"(|)""#)},
        //special_case_add_dollarsign = {str_repr(r#"$"(|)""#), str_repr(r#"$"($|)""#)},
    )]
    fn should_add_interpolation(before: CmdLine, expected: CmdLine) {
        pretty_assertions::assert_eq!(dwim_interpolate_cli(before).unwrap(), expected);
    }

    #[test]
    fn cli_helper() {
        pretty_assertions::assert_eq!(
            str_repr("h|ello world"),
            CmdLine {
                text: b"hello world".to_vec(),
                cursor_pos: 1,
            }
        )
    }
}

mod cmd_line {
    use std::cmp;

    use super::*;
    #[derive(Clone, PartialEq)]
    pub struct CmdLine {
        pub text: Vec<u8>,
        pub cursor_pos: usize,
    }

    impl CmdLine {
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
    }

    #[cfg(test)]
    mod impl_tests {
        use super::*;
        use yare::parameterized;
        mod push_cursor {
            use super::*;
            #[parameterized(
                before_cursor = {str_repr(" |  "), 0, b'i', str_repr("i |  ")},
                at_cursor     = {str_repr(" |  "), 1, b'i', str_repr(" i|  ")},
                after_cursor  = {str_repr(" |  "), 2, b'i', str_repr(" | i ")},
                at_end        = {str_repr(" |  "), 3, b'i', str_repr(" |  i")},
            )]
            fn should_correctly_adjust_cursor_pos_inserting_character(
                before: CmdLine,
                pos: usize,
                char: u8,
                expected: CmdLine,
            ) {
                let mut actual = before.clone();
                actual.insert_push_cursor(pos, char);
                pretty_assertions::assert_eq!(actual, expected);
            }
        }
        mod no_push_cursor {
            use super::*;
            #[parameterized(
                before_cursor = {str_repr(" |  "), 0, b'i', str_repr("i |  ")},
                at_cursor     = {str_repr(" |  "), 1, b'i', str_repr(" |i  ")},
                after_cursor  = {str_repr(" |  "), 2, b'i', str_repr(" | i ")},
                at_end        = {str_repr(" |  "), 3, b'i', str_repr(" |  i")},
            )]
            fn should_correctly_adjust_cursor_pos_inserting_character(
                before: CmdLine,
                pos: usize,
                char: u8,
                expected: CmdLine,
            ) {
                let mut actual = before.clone();
                actual.insert_no_push_cursor(pos, char);
                pretty_assertions::assert_eq!(actual, expected);
            }
        }
    }

    impl fmt::Debug for CmdLine {
        fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
            fmt.debug_tuple("CmdLine")
                .field(&String::from_utf8_lossy(&to_str_repr(self.clone())).as_ref())
                .finish()
        }
    }

    pub fn str_repr<ByteArr: AsRef<[u8]> + std::fmt::Display>(lit: ByteArr) -> CmdLine {
        let bytes = lit.as_ref();
        for (idx, char) in bytes.iter().enumerate() {
            if *char == b'|' {
                let mut ret = Vec::with_capacity(bytes.len() - 1);
                ret.extend_from_slice(&bytes[0..idx]);
                ret.extend_from_slice(&bytes[idx + 1..bytes.len()]);
                return CmdLine {
                    text: ret,
                    cursor_pos: idx,
                };
            }
        }
        panic!("No '|' in `{}`", &lit);
    }

    pub fn to_str_repr(cmd_line: CmdLine) -> Vec<u8> {
        let mut ret = Vec::with_capacity(cmd_line.text.len() + 1);
        for idx in 0..(cmd_line.text.len() + 1) {
            ret.push(match idx.cmp(&cmd_line.cursor_pos) {
                Ordering::Less => cmd_line.text[idx],
                Ordering::Equal => b'|',
                Ordering::Greater => cmd_line.text[idx - 1],
            })
        }
        ret
    }

    #[cfg(test)]
    mod str_repr_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn roundtrip(
                (text, cursor_pos) in "[^|]*".prop_flat_map(|str| {
                    let bytes = str.as_bytes();
                    (Just(bytes.to_owned()), 0..(bytes.len() + 1))
                })
            ) {
                let original = CmdLine { cursor_pos, text };
                let round_tripped = str_repr(
                    unsafe {
                        String::from_utf8_unchecked(to_str_repr(original.clone()))
                    }
                );
                prop_assert_eq!(original, round_tripped);
            }
        }
    }
}
