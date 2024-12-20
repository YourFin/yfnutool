/// Comptime lookup of a node kind symbol id
/// in the nu tree sitter grammar
macro_rules! nu_kind_sym {
    ($str:expr) => {
        comptime::comptime! {
            const NAMED_SYMBOL: bool = true;
            let lang: tree_sitter::Language = tree_sitter_nu::LANGUAGE.into();
            let sym = lang.id_for_node_kind($str, NAMED_SYMBOL);
            assert!(sym != 0, "nu_kind_sym: No symbol named {}", $str);
            sym
        }
    };
}
pub(crate) use nu_kind_sym;
