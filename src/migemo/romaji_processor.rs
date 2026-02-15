
use std::collections::{HashMap, HashSet};

use crate::migemo::bit_list::BitList;
use crate::migemo::bit_vector::BitVector;
use crate::migemo::louds_trie::LoudsTrie;

#[derive(Clone, Default)]
struct MappingEntry {
    value_index: u16,
    remain: u8,
}

pub struct RomajiProcessor {
    key_trie: LoudsTrie,
    key_terminals: BitVector,
    value_trie: LoudsTrie,
    node_mappings: Vec<Option<MappingEntry>>,
}

pub struct RomajiPredictiveResult {
    pub prefix: Vec<u16>,
    pub suffixes: Vec<Vec<u16>>,
}

impl RomajiProcessor {
    pub fn new() -> RomajiProcessor {
        let key_bitvector = BitVector::new(ROMAJI_KEY_TRIE_BITS.to_vec(), ROMAJI_KEY_TRIE_SIZE);
        let key_trie = LoudsTrie {
            bit_vector: key_bitvector,
            edges: ROMAJI_KEY_TRIE_EDGES.encode_utf16().collect(),
        };
        let key_terminals = BitVector::new(ROMAJI_KEY_TERMINAL_BITS.to_vec(), ROMAJI_KEY_TERMINAL_SIZE);
        let value_bitvector = BitVector::new(ROMAJI_VALUE_TRIE_BITS.to_vec(), ROMAJI_VALUE_TRIE_SIZE);
        let value_trie = LoudsTrie {
            bit_vector: value_bitvector,
            edges: ROMAJI_VALUE_TRIE_EDGES.encode_utf16().collect(),
        };
        let mut node_mappings = vec![None; key_terminals.size()];
        let mut mapping_idx = 0;
        debug_assert_eq!(ROMAJI_MAPPING_INDEX.len(), ROMAJI_MAPPING_REMAIN.len());
        for node_index in 0..node_mappings.len() {
            if !key_terminals.get(node_index) {
                continue;
            }
            if mapping_idx >= ROMAJI_MAPPING_INDEX.len() {
                break;
            }
            node_mappings[node_index] = Some(MappingEntry {
                value_index: ROMAJI_MAPPING_INDEX[mapping_idx],
                remain: ROMAJI_MAPPING_REMAIN[mapping_idx],
            });
            mapping_idx += 1;
        }
        RomajiProcessor {
            key_trie,
            key_terminals,
            value_trie,
            node_mappings,
        }
    }

    pub fn romaji_to_hiragana(&self, romaji: &str) -> String {
        if romaji.is_empty() {
            return String::new();
        }
        let romaji_utf16: Vec<u16> = romaji.encode_utf16().collect();

        let mut hiragana = Vec::with_capacity(romaji_utf16.len());
        let mut value_buffer: Vec<u16> = Vec::with_capacity(4);
        let mut start = 0;

        while start < romaji_utf16.len() {
            let query = &romaji_utf16[start..];
            let mut best_match: Option<(usize, usize)> = None;
            for (len, node_index) in self.key_trie.common_prefix_search(query).enumerate() {
                let len = len + 1;
                if self.key_terminals.get(node_index) {
                    best_match = Some((node_index, len));
                }
            }

            if let Some((node_index, match_len)) = best_match {
                if let Some(entry) = self.mapping_entry(node_index) {
                    self.value_trie
                        .get_key_into(entry.value_index as usize, &mut value_buffer);
                    hiragana.extend_from_slice(&value_buffer);
                    start += match_len - entry.remain as usize;
                    continue;
                }
            }

            hiragana.push(romaji_utf16[start]);
            start += 1;
        }

        String::from_utf16(&hiragana).unwrap_or_default()
    }

    fn mapping_entry(&self, node_index: usize) -> Option<&MappingEntry> {
        self.node_mappings
            .get(node_index)
            .and_then(|entry| entry.as_ref())
    }

    fn terminal_nodes_for_prefix(&self, prefix: &[u16]) -> Vec<usize> {
        if let Some(node_index) = self.key_trie.get(prefix) {
            self.key_trie
                .predictive_search(node_index)
                .filter(|&index| index < self.key_terminals.size() && self.key_terminals.get(index))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn romaji_to_hiragana_predictively(&self, romaji: &[u16]) -> RomajiPredictiveResult {
        if romaji.is_empty() {
            return RomajiPredictiveResult {
                prefix: vec![],
                suffixes: vec![vec![]],
            };
        }
        let mut hiragana = Vec::new();
        let mut value_buffer: Vec<u16> = Vec::with_capacity(4);
        let mut start = 0;

        while start < romaji.len() {
            let query = &romaji[start..];
            let mut last_found: Option<(usize, usize)> = None;
            for (len, node_index) in self.key_trie.common_prefix_search(query).enumerate() {
                let len = len + 1;
                if self.key_terminals.get(node_index) {
                    last_found = Some((node_index, len));
                }
            }

            let terminal_nodes = self.terminal_nodes_for_prefix(query);
            if terminal_nodes.len() > 1 {
                let mut set: HashSet<Vec<u16>> = HashSet::new();
                for node_index in terminal_nodes {
                    let entry = match self.mapping_entry(node_index) {
                        Some(entry) => entry,
                        None => continue,
                    };
                    if entry.remain > 0 {
                        let remain = entry.remain as usize;
                        if romaji.len() >= remain {
                            let offset = romaji.len() - remain;
                            let next_nodes = self.terminal_nodes_for_prefix(&romaji[offset..]);
                            for next_node in next_nodes {
                                let next_entry = match self.mapping_entry(next_node) {
                                    Some(entry) => entry,
                                    None => continue,
                                };
                                if next_entry.remain == 0 {
                                    self.value_trie
                                        .get_key_into(entry.value_index as usize, &mut value_buffer);
                                    let mut combined = value_buffer.clone();
                                    self.value_trie.get_key_into(
                                        next_entry.value_index as usize,
                                        &mut value_buffer,
                                    );
                                    combined.extend_from_slice(&value_buffer);
                                    set.insert(combined);
                                }
                            }
                        }
                    } else {
                        self.value_trie
                            .get_key_into(entry.value_index as usize, &mut value_buffer);
                        set.insert(value_buffer.clone());
                    }
                }
                return RomajiPredictiveResult {
                    prefix: hiragana,
                    suffixes: set.into_iter().collect(),
                };
            }

            if let Some((node_index, match_len)) = last_found {
                if let Some(entry) = self.mapping_entry(node_index) {
                    self.value_trie
                        .get_key_into(entry.value_index as usize, &mut value_buffer);
                    hiragana.extend_from_slice(&value_buffer);
                    start += match_len - entry.remain as usize;
                    continue;
                }
            }

            hiragana.push(romaji[start]);
            start += 1;
        }

        RomajiPredictiveResult {
            prefix: hiragana,
            suffixes: vec![vec![]],
        }
    }

}

pub fn build(roman_entries: &[(&str, &str, usize)]) {
    // ローマ字->(ひらがな,送り文字数)の辞書を構築
    let mut dict = HashMap::<&str, (&str, usize)>::new();
    for (roman_str, hiragana_str, remain) in roman_entries.iter() {
        dict.insert(*roman_str, (*hiragana_str, *remain));
    }
    // ローマ字を格納したLoudsTrieを構築
    let mut keys = roman_entries.iter()
        .map(|(roman_str, _, _)| roman_str.encode_utf16().collect::<Vec<u16>>())
        .collect::<Vec<Vec<u16>>>();
    keys.sort();
    let (key_dict, _key_index) = LoudsTrie::build(&keys);
    // ひらがなを格納したLoudsTrieを構築
    let mut values = roman_entries.iter()
        .map(|(_, hiragana_str, _)| hiragana_str.encode_utf16().collect::<Vec<u16>>())
        .collect::<Vec<Vec<u16>>>();
    values.sort();
    let (value_dict, _) = LoudsTrie::build(&values);
    // キーのビットリストとマッピングを構築
    // key_indexはノードインデックスを含むが、edgesのインデックスとして使う必要がある
    // LoudsTrieではノードインデックスはedgesインデックスと同じではない
    // しかし、この実装では各キーの終端ノードをedgesインデックスとして扱う必要がある
    let mut terminal_flags = BitList::new_with_size(key_dict.edges.len());
    for i in 0..keys.len() {
        let terminal_flags_vec = key_dict.get(&keys[i]).unwrap();
        terminal_flags.set(terminal_flags_vec, true);
    }
    let key_bitvec = BitVector::new(
        terminal_flags.words().to_vec(),
        terminal_flags.len(),
    );
    let mut mapping = vec![(0u16, 0u8); roman_entries.len()];
    for (roman, hiragana, remain) in roman_entries.iter() {
        let roman_utf16: Vec<u16> = roman.encode_utf16().collect();
        let key_id = key_dict.get(&roman_utf16).unwrap();
        // rank returns count of bits set to true before pos (not including pos)
        // so we use rank(key_id + 1, true) to include key_id
        let key_bits_index = key_bitvec.rank(key_id + 1, true);
        if key_bits_index == 0 {
            panic!("key_bits_index is 0 for roman: {}, key_id: {}", roman, key_id);
        }
        let hiragana_utf16: Vec<u16> = hiragana.encode_utf16().collect();
        let value_id = value_dict.get(&hiragana_utf16).unwrap();
        mapping[key_bits_index - 1] = (
            value_id as u16,
            *remain as u8,
        );
    }
    // 辞書を出力
    println!("Key LoudsTrie size: {}", key_dict.bit_vector.size());
    println!("Key LoudsTrie bit vector: {:?}", key_dict.bit_vector.words());
    println!("Key LoudsTrie edges: {:?}", key_dict.edges);
    println!(
        "Key LoudsTrie as string: {}",
        key_dict
            .edges
            .iter()
            .filter_map(|&x| char::from_u32(x as u32))
            .collect::<String>()
    );
    println!("Value LoudsTrie size: {}", value_dict.size());
    println!("Value LoudsTrie bit vector: {:?}", value_dict.bit_vector.words());
    println!("Value LoudsTrie edges: {:?}", value_dict.edges);
       println!(
        "Value LoudsTrie as string: {}",
        value_dict
            .edges
            .iter()
            .filter_map(|&x| char::from_u32(x as u32))
            .collect::<String>()
    );
    println!("Key bits size: {:?}", key_bitvec.size());
    println!("Key bits: {:?}", key_bitvec.words());
    println!("Mapping: {:?}", mapping.iter().map(|(v, _)| *v).collect::<Vec<u16>>());
    println!("Offset: {:?}", mapping.iter().map(|(_, o)| *o).collect::<Vec<u8>>());
}

const ROMAJI_KEY_TRIE_EDGES: &str = "\0\0,-.[]abcdefghijklmnopqrstuvwxyz~abeiouyacehiouy'adehiouwyaefiouyaegiouwyaehiouwyaeijouyaeikouwyaeiklotuwyaeimouy'aeinouyaeiopuyaeioquaeioruyaehiosuy'aehiostuwyaeiouvyaehiouwyaeiknotuwxyaeouy,-./[]aehijklouyzaeiouaeiouaeiouiuyaeiouaeiouaeiouaouaeiouaeiouaeioyaeiouaeiouaeiouaeiouaesuaaeiouaeiouaeiouaeiouaeiouaeiouaeiouiuyaeiouaeiouaeiouaeiouaeiouaeiouweiaesuaaeiouaeiouuuuuu";
const ROMAJI_KEY_TRIE_SIZE: usize = 750;
const ROMAJI_KEY_TRIE_BITS: [u64; 12] = [
    18302345228813598717,
    17293258513191002063,
    17581982438530195439,
    2234066889061953527,
    17301712523070241854,
    1764830644966768765,
    16141173809959667667,
    16158626261263942627,
    17293823104241583075,
    18014398510530561,
    36028797018980352,
    268435456,
];
const ROMAJI_VALUE_TRIE_EDGES: &str = "\0\0w‥…←↑→↓、。「」『』〜ぁあぃいぅうぇえぉおかがきぎくぐけげこごさざしじすずせぜそぞただちぢっつづてでとどなにぬねのはばぱひびぴふぶぷへべぺほぼぽまみむめもゃやゅゆょよらりるれろゎわゐゑをんゔヵヶ・ーぇぁぃぇぉぃぇゃゅょぃぇゃゅょぁぃぅぇぉぁぃぅぇぉぃぇゃゅょぃぇゃゅょぃぇゃゅょぃぇゃゅょぁぃぇぉぃぇゃゅょぃぇゃゅょぁぃぅぇぉぁぃぅぇぉぃぇゃゅょぃぇゃゅょぃぇゃゅょぃぇゃゅょぁぃぇぉゃゅょぃぇゃゅょぃぇゃゅょぁぃぇぉゃゅょ";
const ROMAJI_VALUE_TRIE_SIZE: usize = 432;
const ROMAJI_VALUE_TRIE_BITS: [u64; 7] = [
    18446744073709551613,
    16429132540159197183,
    8935695541798952705,
    16103754956413855647,
    571961413348624375,
    0,
    0,
];
const ROMAJI_KEY_TERMINAL_SIZE: usize = 376;
const ROMAJI_KEY_TERMINAL_BITS: [u64; 6] = [18004528185146902780, 18302058713675791613, 18066044213155258366, 18446744056529649663, 18446744073642442743, 72057319160020987];

const ROMAJI_MAPPING_INDEX: [u16; 312] = [9, 103, 10, 11, 12, 17, 23, 19, 98, 25, 21, 15, 63, 50, 72, 66, 75, 69, 26, 50, 42, 38, 34, 30, 47, 50, 54, 49, 56, 52, 193, 195, 50, 194, 196, 68, 27, 33, 50, 29, 35, 31, 62, 71, 50, 65, 74, 68, 136, 135, 39, 50, 138, 137, 26, 32, 28, 50, 34, 30, 16, 22, 18, 50, 24, 20, 77, 80, 78, 50, 81, 79, 98, 57, 60, 58, 98, 61, 59, 64, 73, 67, 76, 50, 70, 119, 122, 120, 123, 50, 30, 88, 91, 89, 92, 50, 90, 36, 42, 38, 44, 50, 40, 46, 53, 48, 55, 50, 51, 210, 212, 211, 213, 99, 50, 94, 107, 106, 97, 21, 50, 16, 22, 18, 98, 24, 20, 50, 83, 104, 87, 85, 50, 3, 15, 4, 102, 13, 14, 37, 43, 5, 39, 8, 6, 7, 45, 41, 50, 185, 184, 183, 187, 186, 141, 140, 48, 143, 142, 141, 140, 139, 143, 142, 158, 170, 160, 159, 158, 162, 161, 168, 171, 169, 172, 170, 146, 145, 144, 148, 147, 197, 199, 198, 124, 127, 125, 128, 126, 116, 115, 114, 118, 117, 193, 195, 194, 196, 180, 179, 178, 182, 181, 136, 135, 134, 138, 137, 119, 122, 120, 123, 121, 111, 110, 109, 113, 112, 100, 101, 50, 93, 82, 22, 18, 86, 84, 202, 201, 200, 204, 203, 175, 174, 173, 177, 176, 190, 189, 188, 192, 191, 207, 206, 205, 209, 208, 131, 130, 38, 133, 132, 131, 130, 129, 133, 132, 153, 165, 155, 154, 153, 157, 156, 149, 151, 150, 152, 51, 163, 166, 164, 167, 165, 141, 140, 139, 143, 142, 214, 212, 211, 216, 215, 105, 107, 106, 108, 21, 2, 96, 95, 100, 101, 50, 93, 82, 22, 18, 86, 84, 136, 135, 134, 138, 137, 161, 198, 50, 156, 50, 0];
const ROMAJI_MAPPING_REMAIN: [u8; 312] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

#[cfg(test)]
mod tests {
    use super::*;

    fn romaji_to_hiragana(romaji: &str) -> String {
        let dict = RomajiProcessor::new();
        dict.romaji_to_hiragana(romaji)
    }

    #[test]
    fn romaji_to_hiragana_1() {
        let actual = romaji_to_hiragana("kensaku");
        let expected = "けんさく";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_2() {
        let actual = romaji_to_hiragana("atti");
        let expected = "あっち";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_3() {
        let actual = romaji_to_hiragana("att");
        let expected = "あっt";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_4() {
        let actual = romaji_to_hiragana("www");
        let expected = "wっw";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_5() {
        let actual = romaji_to_hiragana("kk");
        let expected = "っk";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_6() {
        let actual = romaji_to_hiragana("n");
        let expected = "ん";
        assert_eq!(actual, expected);
    }

    fn romaji_to_hiragana_predictively(romaji: &str) -> (String, Vec<String>) {
        let dict = RomajiProcessor::new();
        let kensaku: Vec<u16> = romaji.encode_utf16().collect();
        let actual = dict.romaji_to_hiragana_predictively(&kensaku);
        let prefix = String::from_utf16(&actual.prefix).unwrap();
        let mut suffixes: Vec<String> = actual
            .suffixes
            .iter()
            .map(|x| String::from_utf16(x).unwrap())
            .collect();
        suffixes.sort();
        (prefix, suffixes)
    }

    #[test]
    fn romaji_to_hiragana_predictively_1() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("kiku");
        assert_eq!(prefix, "きく");
        assert_eq!(suffixes.len(), 1);
        assert_eq!(suffixes[0], "");
    }

    #[test]
    fn romaji_to_hiragana_predictively_2() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("ky");
        let mut expected_suffixes = vec!["きゃ", "きぃ", "きぇ", "きゅ", "きょ"];
        expected_suffixes.sort();
        assert_eq!(prefix, "");
        assert_eq!(suffixes, expected_suffixes);
    }

    #[test]
    fn romaji_to_hiragana_predictively_3() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("kky");
        let mut expected_suffixes = vec!["きゃ", "きぃ", "きぇ", "きゅ", "きょ"];
        expected_suffixes.sort();
        assert_eq!(prefix, "っ");
        assert_eq!(suffixes, expected_suffixes);
    }

    #[test]
    fn romaji_to_hiragana_predictively_4() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("n");
        let mut expected_suffixes = vec![
            "にょ", "の", "にゃ", "ぬ", "ね", "な", "にぇ", "にゅ", "に", "ん", "にぃ",
        ];
        expected_suffixes.sort();
        assert_eq!(prefix, "");
        assert_eq!(suffixes, expected_suffixes);
    }

    #[test]
    fn romaji_to_hiragana_predictively_5() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("denk");
        assert_eq!(prefix, "でん");
        assert!(suffixes.iter().any(|s| s == "か"));
    }

    #[test]
    fn romaji_to_hiragana_predictively_w() {
        let (_, _) = romaji_to_hiragana_predictively("w");
    }
}
