extern crate proc_macro;

use proc_macro::{Literal, TokenStream, TokenTree};
use syn::parse_macro_input;
use tree_sitter;
use tree_sitter_nu;

const NAMED_SYMBOL: bool = true;

/// Comptime lookup of a node kind symbol id
/// in the nu tree sitter grammar
///
/// Takes a string literal as an argument
#[proc_macro]
pub fn nu_kind_sym(input: TokenStream) -> TokenStream {
    let node_kind = parse_macro_input!(input as syn::LitStr).value();
    let lang: tree_sitter::Language = tree_sitter_nu::LANGUAGE.into();
    let sym = lang.id_for_node_kind(&node_kind, NAMED_SYMBOL);
    assert!(
        sym != 0,
        "nu_kind_sym: No kind symbol named {} in nu tree-sitter grammar",
        node_kind
    );
    TokenStream::from(TokenTree::from(Literal::u16_suffixed(sym)))
}
