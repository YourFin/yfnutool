use std::io::IsTerminal;

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, info, warn};
use tree_sitter::{InputEdit, Language, Point};

#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

#[derive(Debug, PartialEq, Eq)]
enum ParseState {
    SingleQuote,
    DoubleQuote,
    RawString,
    // Ignoring bare-word strings
    Backtick,
    SingleQuoteInterpolated,
    DoubleQuoteInterpolated,

    // "not in the middle of a string"
    Other,

    DollarSign,
    RawStringInR,
    RawStringInPound,
    RawStringInSingle,
}

// h r#'
// ^ boring
//  ^ boring
//   ^ maybe an r#' ?
//    ^ maybe an r#' ?
//     ^ An r#'!

#[derive(Debug, PartialEq)]
struct CmdLine<'a> {
    text: &'a [u8],
    cursor_pos: usize,
}

fn get_new_commandline(cmd_line: CmdLine) -> Result<CmdLine> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_nu::LANGUAGE.into())
        .with_context(|| "Error loading nu grammar")?;

    Ok(CmdLine {
        text: b"",
        cursor_pos: 0,
    })
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();
    info!("starting up");
    warn!("weewooweewoo");
    let is_terminal = std::io::stdout().is_terminal();
    debug!("std::io::stdout().is_terminal: {}", is_terminal);
    let path = "test.txt";
    let content =
        std::fs::read_to_string(path).with_context(|| format!("could not read file `{}`", path))?;
    println!("file content: {}", content);
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn should_add_interpolation() {
        let arena = bumpalo::Bump::new();
        let test_cases = vec![(cli(r#"foo "ba| "#), cli(r#"foo $"ba| "#))];
        for (before, expected) in test_cases {
            assert_eq!(
                get_new_commandline(before(&arena)).unwrap(),
                expected(&arena)
            );
        }
    }

    // Helper
    fn cli<'bump>(lit: &'static str) -> impl FnOnce(&'bump bumpalo::Bump) -> CmdLine<'bump> {
        move |bump| {
            let bytes = lit.as_bytes();
            for (idx, char) in bytes.iter().enumerate() {
                if *char == b'|' {
                    let mut ret =
                        bumpalo::boxed::Box::new_in(bumpalo::collections::Vec::new_in(bump), bump);
                    ret.extend_from_slice(&bytes[0..idx]);
                    ret.extend_from_slice(&bytes[idx + 1..lit.len()]);
                    return CmdLine {
                        text: bumpalo::boxed::Box::leak(ret),
                        cursor_pos: idx,
                    };
                }
            }
            panic!("No '|' in `{}`", &lit);
        }
    }

    #[test]
    fn cli_helper() {
        let arena = bumpalo::Bump::new();
        assert_eq!(
            cli("h|ello world")(&arena),
            CmdLine {
                text: b"hello world",
                cursor_pos: 1
            }
        )
    }
}
