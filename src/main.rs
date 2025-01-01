//use std::io::IsTerminal;

mod cmd_line;
use cmd_line::ToStrRepr;
mod nu_kind_sym;

use anyhow::{Context, Result};
use clap::Parser;
use log::{debug, log_enabled, trace};
use nu_kind_sym::nu_kind_sym;
#[cfg(test)]
use pretty_assertions::{assert_eq, assert_ne};
use tree_sitter::TreeCursor;
//use tree_sitter::{InputEdit, Language, Point};

// Dummy wrapper to implement "Orphan" instances
struct Id<T>(T);

#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
    #[arg(long)]
    test_string: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    env_logger::Builder::new()
        .filter_level(cli.verbose.log_level_filter())
        .init();
    match cli.test_string {
        Some(str) => {
            let cmd_line: cmd_line::Bytes = cmd_line::str_repr(str.clone());
            let result = dwim_interpolate_cli(cmd_line)
                .with_context(|| format!("Error running against {:?}", str))?;

            println!("{}", result.to_str_repr());
        }
        None => {
            let (cursor_pos_grapheme, text) = rmp_serde::decode::from_read(std::io::stdin())
                .with_context(|| "Unable to read from stdin")?;
            let bytes_cli = dwim_interpolate_cli(
                cmd_line::Utf8 {
                    text,
                    cursor_pos_grapheme,
                }
                .into(),
            )?;
            let utf8_cli: cmd_line::Utf8 = bytes_cli
                .try_into()
                .with_context(|| "dwim_interpolate_cli returned invalid utf8")?;
            rmp_serde::encode::write(
                &mut std::io::stdout(),
                &(utf8_cli.cursor_pos_grapheme, utf8_cli.text),
            )?;
        }
    }
    Ok(())
}

// h r#'
// ^ boring
//  ^ boring
//   ^ maybe an r#' ?
//    ^ maybe an r#' ?
//     ^ An r#'!

fn dwim_interpolate_cli(mut input: cmd_line::Bytes) -> Result<cmd_line::Bytes> {
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
        .named_descendant_for_byte_range(effective_cursor_pos, effective_cursor_pos)
        .with_context(|| {
            format!(
                "Unable to find node at cursor position {}",
                effective_cursor_pos
            )
        })?;
    if log_enabled!(log::Level::Debug) {
        let mut node = innermost_node;
        'do_while: while {
            /* do */
            debug!("node parent chain: {} ({})", node.kind(), node.kind_id());
            let cont = node.id() != tree.root_node().id();
            let optional_node = node.parent();
            node = match optional_node {
                Some(wrapped) => wrapped,
                None => break 'do_while,
            };
            /* while */
            cont
        } {}
    }
    if log_enabled!(log::Level::Debug) {
        use annotate_snippets::{Level, Renderer, Snippet};
        let mut with_depths: Vec<_> = preorder_iter(&tree).collect();
        with_depths.sort_by_key(|(depth, _node)| *depth);
        let src = String::from_utf8_lossy(input.text.as_ref());
        let msg_arena = bumpalo::Bump::new();
        let mut message = Level::Info.title("Tree sitter parse results:").snippet(
            Snippet::source(src.as_ref()).line_start(1).annotations(
                with_depths
                    .iter()
                    .enumerate()
                    .rev()
                    .map(|(node_idx, (_depth, node))| {
                        Level::Info
                            .span(node.byte_range())
                            .label(bumpalo::boxed::Box::leak(bumpalo::boxed::Box::new_in(
                                bumpalo::format!(in &msg_arena,
                                                 "{:X} ({}): {}",
                                    node.id(),
                                                 node_idx,
                                    node.kind(),
                                ),
                                &msg_arena,
                            )))
                    }),
            ),
        );
        let renderer = Renderer::styled();
        if log_enabled!(log::Level::Trace) {
            message =
                message.footers(with_depths.iter().enumerate().rev().map(
                    |(node_idx, (_depth, node))| {
                        Level::Help
                            .title(bumpalo::boxed::Box::leak(bumpalo::boxed::Box::new_in(
                                bumpalo::format!(in &msg_arena,
                                                 "{:X} ({}): {} (kind_id: {})",
                                    node.id(),
                                    node_idx,
                                    node.kind(),
                                    node.kind_id()
                                ),
                                &msg_arena,
                            )))
                            .snippet(Snippet::source(src.as_ref()).fold(true).annotation(
                                Level::Help.span(node.byte_range()).label(
                                    bumpalo::boxed::Box::leak(bumpalo::boxed::Box::new_in(
                                        bumpalo::format!(in &msg_arena,
                                                         "here (byte {}-{})",
                                            node.byte_range().start,
                                            node.byte_range().end,
                                        ),
                                        &msg_arena,
                                    )),
                                ),
                            ))
                    },
                ));
            trace!("{}", renderer.render(message));
        } else {
            debug!("{}", renderer.render(message));
        }
    }

    match (
        innermost_node.kind_id(),
        input.text[innermost_node.start_byte()],
    ) {
        (nu_kind_sym!("val_string") | nu_kind_sym!("ERROR"), b'\'') => {
            debug!("Single quote string");
            input = escape::prep_interpolate_single(input, innermost_node.byte_range());
            input.insert_push_cursor(innermost_node.start_byte(), b'$');
            input.insert_push_cursor(input.cursor_pos, b'(');
            input.insert_no_push_cursor(input.cursor_pos, b')');
            return Ok(input);
        }
        //(nu_kind_sym!("val_string"), b'\"') => {
        //    debug!("Double quote string");
        //    input.insert_push_cursor(innermost_node.start_byte(), b'$');
        //    input.insert_push_cursor(input.cursor_pos, b'(');
        //    input.insert_no_push_cursor(input.cursor_pos, b')');
        //    return Ok(input);
        //}
        _ => (),
    }

    Ok(cmd_line::Bytes {
        text: vec![].into(),
        cursor_pos: 0,
    })
}

fn preorder_iter<'tree>(
    tree: &'tree tree_sitter::Tree,
) -> impl Iterator<Item = (u32, tree_sitter::Node<'tree>)> + use<'tree> {
    enum State {
        Unexplored,
        ExploredChildren,
        ExploredAllSiblings,
    }
    use State::*;
    struct I<'a> {
        cursor: tree_sitter::TreeCursor<'a>,
        state: State,
    }
    impl<'a> Iterator for I<'a> {
        type Item = (u32, tree_sitter::Node<'a>);
        fn next(&mut self) -> Option<Self::Item> {
            match self.state {
                Unexplored => {
                    while self.cursor.goto_first_child() {}
                    self.state = ExploredChildren;
                    self.next()
                }
                ExploredChildren => {
                    let ret = (self.cursor.depth(), self.cursor.node());
                    if self.cursor.goto_next_sibling() {
                        self.state = Unexplored;
                    } else {
                        self.state = ExploredAllSiblings;
                    }
                    Some(ret)
                }
                ExploredAllSiblings => {
                    if self.cursor.goto_parent() {
                        self.state = ExploredChildren;
                        self.next()
                    } else {
                        None
                    }
                }
            }
        }
    }
    I {
        cursor: tree.walk(),
        state: Unexplored,
    }
}

mod escape {
    use super::*;
    use std::ops::Range;
    fn double_to_double_interpolate(
        mut cmd_line: cmd_line::Bytes,
        range: Range<usize>,
    ) -> cmd_line::Bytes {
        let mut idx = range.start;
        while idx < range.end {
            match cmd_line.text[idx] {
                b'(' => {
                    cmd_line.insert_push_cursor(idx, b'\\');
                    idx += 1;
                }
                _ => (),
            }

            idx += 1;
        }
        cmd_line
    }

    // TODO: think about this more
    fn single_to_double(mut cmd_line: cmd_line::Bytes, range: Range<usize>) -> cmd_line::Bytes {
        let mut idx = range.start;
        while idx < range.end {
            match cmd_line.text[idx] {
                b'"' => {
                    cmd_line.insert_push_cursor(idx, b'\\');
                    idx += 1;
                }
                _ => (),
            }

            idx += 1;
        }
        cmd_line
    }

    pub fn prep_interpolate_single(
        mut cmd_line: cmd_line::Bytes,
        interpolate_range: Range<usize>,
    ) -> cmd_line::Bytes {
        let mut idx = interpolate_range.start;
        while idx < interpolate_range.end {
            match cmd_line.text[idx] {
                b'(' => {
                    const REPLACEMENT: &[u8] = br#"('(')"#;
                    cmd_line.overwrite_range(idx..idx + 1, REPLACEMENT);
                    idx += REPLACEMENT.len();
                }
                _ => {
                    idx += 1;
                }
            }
        }
        cmd_line
    }

    //fn double_to_single(mut cmd_line: CmdLine, range: Range<usize>) -> CmdLine {
    //    let mut idx = range.start;
    //    while idx < range.end {
    //        match cmd_line.text[idx] {
    //            b'\\' => {
    //                cmd_line.delete_no_pull_cursor(idx);
    //                match cmd_line.text[idx] {
    //                    b'(' => {
    //                    }
    //                }
    //            }
    //            _ => (),
    //        }
    //    }
    //}
}

#[cfg(test)]
mod tests {
    use super::*;
    use cmd_line::str_repr;
    use yare::parameterized;

    mod single_quote {
        use super::*;
        #[parameterized(
            //in_existing_double_quote_string = {str_repr(r#"foo "ba| "#), str_repr(r#"foo $"ba(|)"#)},
            simple = {str_repr(r#"'|'"#), str_repr(r#"$'(|)'"#)},
            later_in_cli = {str_repr(r#"foo 'ba| '"#), str_repr(r#"foo $'ba(|) '"#)},
            escape_existing_paren = {str_repr(r#"'(ba| '"#), str_repr(r#"$'('(')ba(|) '"#)},
            //empty_string = {str_repr("|"), str_repr(r#"$"(|)""#)},
            //special_case_add_dollarsign = {str_repr(r#"$"(|)""#), str_repr(r#"$"($|)""#)},
        )]
        fn should_add_interpolation(before: cmd_line::Bytes, expected: cmd_line::Bytes) {
            pretty_assertions::assert_eq!(dwim_interpolate_cli(before).unwrap(), expected);
        }
    }

    #[test]
    fn cli_helper() {
        pretty_assertions::assert_eq!(
            str_repr::<_, cmd_line::Bytes>("h|ello world"),
            cmd_line::Bytes {
                text: b"hello world".to_vec(),
                cursor_pos: 1,
            }
        )
    }
}
