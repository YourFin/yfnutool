use std::fmt::Display;

pub fn pretty_print_tree<'tree, 'src>(
    src: &'src str,
    tree: &'tree tree_sitter::Tree,
) -> impl std::fmt::Display + use<'tree, 'src> {
    TreePrinter {
        src,
        tree,
        show_node_details: false,
    }
}
pub fn pretty_print_tree_details<'tree, 'src>(
    src: &'src str,
    tree: &'tree tree_sitter::Tree,
) -> impl std::fmt::Display + use<'tree, 'src> {
    TreePrinter {
        src,
        tree,
        show_node_details: true,
    }
}

struct TreePrinter<'src, 'tree> {
    src: &'src str,
    tree: &'tree tree_sitter::Tree,
    show_node_details: bool,
}
impl<'src, 'tree> Display for TreePrinter<'src, 'tree> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use annotate_snippets::{Level, Renderer, Snippet};
        let mut with_depths: Vec<_> = preorder_depth_first_traverse_iter(&self.tree).collect();
        with_depths.sort_by_key(|(depth, _node)| *depth);

        let msg_arena = bumpalo::Bump::new();
        let mut message = Level::Info.title("Tree sitter parse results:").snippet(
            Snippet::source(self.src).line_start(1).annotations(
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
        if self.show_node_details {
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
                            .snippet(Snippet::source(self.src).fold(true).annotation(
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
        }
        let ret = renderer.render(message).fmt(f);
        ret
    }
}

fn preorder_depth_first_traverse_iter<'tree>(
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
