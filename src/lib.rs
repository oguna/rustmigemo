mod migemo;

extern crate wasm_bindgen;

use wasm_bindgen::prelude::*;
use migemo::compact_dictionary::*;
use migemo::query::*;
use migemo::regex_generator::*;

#[wasm_bindgen]
#[derive(Debug)]
pub struct Migemo {
    dictionary: CompactDictionary,
}

#[wasm_bindgen]
impl Migemo {
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