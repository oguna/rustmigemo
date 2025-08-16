pub mod migemo;

// 以下のモジュール全体が、"wasm" feature が有効な時にのみコンパイルされます。
#[cfg(feature = "wasm")]
mod wasm_exports {
    // wasmビルド時に必要なものをこのモジュール内でインポートします。
    // 親モジュール(crateルート)からインポートするため `super::` を付けます。
    use super::migemo::{compact_dictionary::CompactDictionary, regex_generator::RegexOperator, query::query};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    #[derive(Debug)]
    pub struct Migemo {
        dictionary: CompactDictionary,
    }

    #[wasm_bindgen]
    impl Migemo {
        #[wasm_bindgen(constructor)]
        pub fn new(buffer: &[u8]) -> Migemo {
            let vec = Vec::from(buffer);
            Migemo {
                dictionary: CompactDictionary::new(&vec),
            }
        }

        pub fn query(&self, word: String) -> String {
            let rxop = RegexOperator::Default;
            query(word, &self.dictionary, &rxop)
        }
    }
}

// wasm_exports モジュールの中身（Migemoなど）をライブラリのトップレベルに公開します。
// これも "wasm" feature が有効な時のみ行われます。
#[cfg(feature = "wasm")]
pub use wasm_exports::*;
