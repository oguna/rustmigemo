
use std::collections::HashMap;

use crate::migemo::bit_list::BitList;
use crate::migemo::bit_vector::BitVector;
use crate::migemo::louds_trie::LoudsTrie;

#[derive(Clone, Default)]
struct MappingEntry {
    value_index: u16,
    remain: u8,
}

pub struct RomajiDictionary {
    key_trie: LoudsTrie,
    key_terminals: BitVector,
    value_trie: LoudsTrie,
    node_mappings: Vec<Option<MappingEntry>>,
}

impl RomajiDictionary {
    pub fn new() -> RomajiDictionary {
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
        RomajiDictionary {
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

    //pub fn romaji_to_hiragana_predictively(&self, romaji: &[u16]) -> RomajiPredictiveResult {
    //}

}

pub fn build() {
    // ローマ字->(ひらがな,送り文字数)の辞書を構築
    let mut dict = HashMap::<&str, (&str, usize)>::new();
    for (roman_str, hiragana_str, remain) in ROMAN_ENTRIES.iter() {
        dict.insert(*roman_str, (*hiragana_str, *remain));
    }
    // ローマ字を格納したLoudsTrieを構築
    let mut keys = ROMAN_ENTRIES.iter()
        .map(|(roman_str, _, _)| roman_str.encode_utf16().collect::<Vec<u16>>())
        .collect::<Vec<Vec<u16>>>();
    keys.sort();
    let (key_dict, _key_index) = LoudsTrie::build(&keys);
    // ひらがなを格納したLoudsTrieを構築
    let mut values = ROMAN_ENTRIES.iter()
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
    let mut mapping = vec![(0u16, 0u8); ROMAN_ENTRIES.len()];
    for (roman, hiragana, remain) in ROMAN_ENTRIES.iter() {
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
const ROMAJI_VALUE_TRIE_SIZE: usize = 215;
const ROMAJI_VALUE_TRIE_BITS: [u64; 4] = [
    18446744073709551613, 16429132540159197183, 8935695541798952705, 16103754956413855647
];
const ROMAJI_KEY_TERMINAL_SIZE: usize = 376;
const ROMAJI_KEY_TERMINAL_BITS: [u64; 6] = [18004528185146902780, 18302058713675791613, 18066044213155258366, 18446744056529649663, 18446744073642442743, 72057319160020987];

const ROMAJI_MAPPING_INDEX: [u16; 312] = [9, 103, 10, 11, 12, 17, 23, 19, 98, 25, 21, 15, 63, 50, 72, 66, 75, 69, 26, 50, 42, 38, 34, 30, 47, 50, 54, 49, 56, 52, 193, 195, 50, 194, 196, 68, 27, 33, 50, 29, 35, 31, 62, 71, 50, 65, 74, 68, 136, 135, 39, 50, 138, 137, 26, 32, 28, 50, 34, 30, 16, 22, 18, 50, 24, 20, 77, 80, 78, 50, 81, 79, 98, 57, 60, 58, 98, 61, 59, 64, 73, 67, 76, 50, 70, 119, 122, 120, 123, 50, 30, 88, 91, 89, 92, 50, 90, 36, 42, 38, 44, 50, 40, 46, 53, 48, 55, 50, 51, 210, 212, 211, 213, 99, 50, 94, 107, 106, 97, 21, 50, 16, 22, 18, 98, 24, 20, 50, 83, 104, 87, 85, 50, 3, 15, 4, 102, 13, 14, 37, 43, 5, 39, 8, 6, 7, 45, 41, 50, 185, 184, 183, 187, 186, 141, 140, 48, 143, 142, 141, 140, 139, 143, 142, 158, 170, 160, 159, 158, 162, 161, 168, 171, 169, 172, 170, 146, 145, 144, 148, 147, 197, 199, 198, 124, 127, 125, 128, 126, 116, 115, 114, 118, 117, 193, 195, 194, 196, 180, 179, 178, 182, 181, 136, 135, 134, 138, 137, 119, 122, 120, 123, 121, 111, 110, 109, 113, 112, 100, 101, 50, 93, 82, 22, 18, 86, 84, 202, 201, 200, 204, 203, 175, 174, 173, 177, 176, 190, 189, 188, 192, 191, 207, 206, 205, 209, 208, 131, 130, 38, 133, 132, 131, 130, 129, 133, 132, 153, 165, 155, 154, 153, 157, 156, 149, 151, 150, 152, 51, 163, 166, 164, 167, 165, 141, 140, 139, 143, 142, 214, 212, 211, 216, 215, 105, 107, 106, 108, 21, 2, 96, 95, 100, 101, 50, 93, 82, 22, 18, 86, 84, 136, 135, 134, 138, 137, 161, 198, 50, 156, 50, 0];
const ROMAJI_MAPPING_REMAIN: [u8; 312] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

const ROMAN_ENTRIES: [(&str, &str, usize); 312] = [
    ("-", "ー", 0),
    ("~", "〜", 0),
    (".", "。", 0),
    (",", "、", 0),
    ("z/", "・", 0),
    ("z.", "…", 0),
    ("z,", "‥", 0),
    ("zh", "←", 0),
    ("zj", "↓", 0),
    ("zk", "↑", 0),
    ("zl", "→", 0),
    ("z-", "〜", 0),
    ("z[", "『", 0),
    ("z]", "』", 0),
    ("[", "「", 0),
    ("]", "」", 0),
    ("va", "ゔぁ", 0),
    ("vi", "ゔぃ", 0),
    ("vu", "ゔ", 0),
    ("ve", "ゔぇ", 0),
    ("vo", "ゔぉ", 0),
    ("vya", "ゔゃ", 0),
    ("vyi", "ゔぃ", 0),
    ("vyu", "ゔゅ", 0),
    ("vye", "ゔぇ", 0),
    ("vyo", "ゔょ", 0),
    ("qq", "っ", 1),
    ("vv", "っ", 1),
    ("ll", "っ", 1),
    ("xx", "っ", 1),
    ("kk", "っ", 1),
    ("gg", "っ", 1),
    ("ss", "っ", 1),
    ("zz", "っ", 1),
    ("jj", "っ", 1),
    ("tt", "っ", 1),
    ("dd", "っ", 1),
    ("hh", "っ", 1),
    ("ff", "っ", 1),
    ("bb", "っ", 1),
    ("pp", "っ", 1),
    ("mm", "っ", 1),
    ("yy", "っ", 1),
    ("rr", "っ", 1),
    ("ww", "っ", 1),
    ("www", "w", 2),
    ("cc", "っ", 1),
    ("kya", "きゃ", 0),
    ("kyi", "きぃ", 0),
    ("kyu", "きゅ", 0),
    ("kye", "きぇ", 0),
    ("kyo", "きょ", 0),
    ("gya", "ぎゃ", 0),
    ("gyi", "ぎぃ", 0),
    ("gyu", "ぎゅ", 0),
    ("gye", "ぎぇ", 0),
    ("gyo", "ぎょ", 0),
    ("sya", "しゃ", 0),
    ("syi", "しぃ", 0),
    ("syu", "しゅ", 0),
    ("sye", "しぇ", 0),
    ("syo", "しょ", 0),
    ("sha", "しゃ", 0),
    ("shi", "し", 0),
    ("shu", "しゅ", 0),
    ("she", "しぇ", 0),
    ("sho", "しょ", 0),
    ("zya", "じゃ", 0),
    ("zyi", "じぃ", 0),
    ("zyu", "じゅ", 0),
    ("zye", "じぇ", 0),
    ("zyo", "じょ", 0),
    ("tya", "ちゃ", 0),
    ("tyi", "ちぃ", 0),
    ("tyu", "ちゅ", 0),
    ("tye", "ちぇ", 0),
    ("tyo", "ちょ", 0),
    ("cha", "ちゃ", 0),
    ("chi", "ち", 0),
    ("chu", "ちゅ", 0),
    ("che", "ちぇ", 0),
    ("cho", "ちょ", 0),
    ("cya", "ちゃ", 0),
    ("cyi", "ちぃ", 0),
    ("cyu", "ちゅ", 0),
    ("cye", "ちぇ", 0),
    ("cyo", "ちょ", 0),
    ("dya", "ぢゃ", 0),
    ("dyi", "ぢぃ", 0),
    ("dyu", "ぢゅ", 0),
    ("dye", "ぢぇ", 0),
    ("dyo", "ぢょ", 0),
    ("tsa", "つぁ", 0),
    ("tsi", "つぃ", 0),
    ("tse", "つぇ", 0),
    ("tso", "つぉ", 0),
    ("tha", "てゃ", 0),
    ("thi", "てぃ", 0),
    ("t'i", "てぃ", 0),
    ("thu", "てゅ", 0),
    ("the", "てぇ", 0),
    ("tho", "てょ", 0),
    ("t'yu", "てゅ", 0),
    ("dha", "でゃ", 0),
    ("dhi", "でぃ", 0),
    ("d'i", "でぃ", 0),
    ("dhu", "でゅ", 0),
    ("dhe", "でぇ", 0),
    ("dho", "でょ", 0),
    ("d'yu", "でゅ", 0),
    ("twa", "とぁ", 0),
    ("twi", "とぃ", 0),
    ("twu", "とぅ", 0),
    ("twe", "とぇ", 0),
    ("two", "とぉ", 0),
    ("t'u", "とぅ", 0),
    ("dwa", "どぁ", 0),
    ("dwi", "どぃ", 0),
    ("dwu", "どぅ", 0),
    ("dwe", "どぇ", 0),
    ("dwo", "どぉ", 0),
    ("d'u", "どぅ", 0),
    ("nya", "にゃ", 0),
    ("nyi", "にぃ", 0),
    ("nyu", "にゅ", 0),
    ("nye", "にぇ", 0),
    ("nyo", "にょ", 0),
    ("hya", "ひゃ", 0),
    ("hyi", "ひぃ", 0),
    ("hyu", "ひゅ", 0),
    ("hye", "ひぇ", 0),
    ("hyo", "ひょ", 0),
    ("bya", "びゃ", 0),
    ("byi", "びぃ", 0),
    ("byu", "びゅ", 0),
    ("bye", "びぇ", 0),
    ("byo", "びょ", 0),
    ("pya", "ぴゃ", 0),
    ("pyi", "ぴぃ", 0),
    ("pyu", "ぴゅ", 0),
    ("pye", "ぴぇ", 0),
    ("pyo", "ぴょ", 0),
    ("fa", "ふぁ", 0),
    ("fi", "ふぃ", 0),
    ("fu", "ふ", 0),
    ("fe", "ふぇ", 0),
    ("fo", "ふぉ", 0),
    ("fya", "ふゃ", 0),
    ("fyu", "ふゅ", 0),
    ("fyo", "ふょ", 0),
    ("hwa", "ふぁ", 0),
    ("hwi", "ふぃ", 0),
    ("hwe", "ふぇ", 0),
    ("hwo", "ふぉ", 0),
    ("hwyu", "ふゅ", 0),
    ("mya", "みゃ", 0),
    ("myi", "みぃ", 0),
    ("myu", "みゅ", 0),
    ("mye", "みぇ", 0),
    ("myo", "みょ", 0),
    ("rya", "りゃ", 0),
    ("ryi", "りぃ", 0),
    ("ryu", "りゅ", 0),
    ("rye", "りぇ", 0),
    ("ryo", "りょ", 0),
    ("n'", "ん", 0),
    ("nn", "ん", 0),
    ("n", "ん", 0),
    ("xn", "ん", 0),
    ("a", "あ", 0),
    ("i", "い", 0),
    ("u", "う", 0),
    ("wu", "う", 0),
    ("e", "え", 0),
    ("o", "お", 0),
    ("xa", "ぁ", 0),
    ("xi", "ぃ", 0),
    ("xu", "ぅ", 0),
    ("xe", "ぇ", 0),
    ("xo", "ぉ", 0),
    ("la", "ぁ", 0),
    ("li", "ぃ", 0),
    ("lu", "ぅ", 0),
    ("le", "ぇ", 0),
    ("lo", "ぉ", 0),
    ("lyi", "ぃ", 0),
    ("xyi", "ぃ", 0),
    ("lye", "ぇ", 0),
    ("xye", "ぇ", 0),
    ("ye", "いぇ", 0),
    ("ka", "か", 0),
    ("ki", "き", 0),
    ("ku", "く", 0),
    ("ke", "け", 0),
    ("ko", "こ", 0),
    ("xka", "ヵ", 0),
    ("xke", "ヶ", 0),
    ("lka", "ヵ", 0),
    ("lke", "ヶ", 0),
    ("ga", "が", 0),
    ("gi", "ぎ", 0),
    ("gu", "ぐ", 0),
    ("ge", "げ", 0),
    ("go", "ご", 0),
    ("sa", "さ", 0),
    ("si", "し", 0),
    ("su", "す", 0),
    ("se", "せ", 0),
    ("so", "そ", 0),
    ("ca", "か", 0),
    ("ci", "し", 0),
    ("cu", "く", 0),
    ("ce", "せ", 0),
    ("co", "こ", 0),
    ("qa", "くぁ", 0),
    ("qi", "くぃ", 0),
    ("qu", "く", 0),
    ("qe", "くぇ", 0),
    ("qo", "くぉ", 0),
    ("kwa", "くぁ", 0),
    ("kwi", "くぃ", 0),
    ("kwu", "くぅ", 0),
    ("kwe", "くぇ", 0),
    ("kwo", "くぉ", 0),
    ("gwa", "ぐぁ", 0),
    ("gwi", "ぐぃ", 0),
    ("gwu", "ぐぅ", 0),
    ("gwe", "ぐぇ", 0),
    ("gwo", "ぐぉ", 0),
    ("za", "ざ", 0),
    ("zi", "じ", 0),
    ("zu", "ず", 0),
    ("ze", "ぜ", 0),
    ("zo", "ぞ", 0),
    ("ja", "じゃ", 0),
    ("ji", "じ", 0),
    ("ju", "じゅ", 0),
    ("je", "じぇ", 0),
    ("jo", "じょ", 0),
    ("jya", "じゃ", 0),
    ("jyi", "じぃ", 0),
    ("jyu", "じゅ", 0),
    ("jye", "じぇ", 0),
    ("jyo", "じょ", 0),
    ("ta", "た", 0),
    ("ti", "ち", 0),
    ("tu", "つ", 0),
    ("tsu", "つ", 0),
    ("te", "て", 0),
    ("to", "と", 0),
    ("da", "だ", 0),
    ("di", "ぢ", 0),
    ("du", "づ", 0),
    ("de", "で", 0),
    ("do", "ど", 0),
    ("xtu", "っ", 0),
    ("xtsu", "っ", 0),
    ("ltu", "っ", 0),
    ("ltsu", "っ", 0),
    ("na", "な", 0),
    ("ni", "に", 0),
    ("nu", "ぬ", 0),
    ("ne", "ね", 0),
    ("no", "の", 0),
    ("ha", "は", 0),
    ("hi", "ひ", 0),
    ("hu", "ふ", 0),
    ("fu", "ふ", 0),
    ("he", "へ", 0),
    ("ho", "ほ", 0),
    ("ba", "ば", 0),
    ("bi", "び", 0),
    ("bu", "ぶ", 0),
    ("be", "べ", 0),
    ("bo", "ぼ", 0),
    ("pa", "ぱ", 0),
    ("pi", "ぴ", 0),
    ("pu", "ぷ", 0),
    ("pe", "ぺ", 0),
    ("po", "ぽ", 0),
    ("ma", "ま", 0),
    ("mi", "み", 0),
    ("mu", "む", 0),
    ("me", "め", 0),
    ("mo", "も", 0),
    ("xya", "ゃ", 0),
    ("lya", "ゃ", 0),
    ("ya", "や", 0),
    ("wyi", "ゐ", 0),
    ("xyu", "ゅ", 0),
    ("lyu", "ゅ", 0),
    ("yu", "ゆ", 0),
    ("wye", "ゑ", 0),
    ("xyo", "ょ", 0),
    ("lyo", "ょ", 0),
    ("yo", "よ", 0),
    ("ra", "ら", 0),
    ("ri", "り", 0),
    ("ru", "る", 0),
    ("re", "れ", 0),
    ("ro", "ろ", 0),
    ("xwa", "ゎ", 0),
    ("lwa", "ゎ", 0),
    ("wa", "わ", 0),
    ("wi", "うぃ", 0),
    ("we", "うぇ", 0),
    ("wo", "を", 0),
    ("wha", "うぁ", 0),
    ("whi", "うぃ", 0),
    ("whu", "う", 0),
    ("whe", "うぇ", 0),
    ("who", "うぉ", 0),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn romaji_to_hiragana(romaji: &str) -> String {
        let dict = RomajiDictionary::new();
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
}
