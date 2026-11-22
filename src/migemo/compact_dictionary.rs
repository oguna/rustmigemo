use super::bit_list::BitList;
use super::bit_vector::BitVector;
use super::louds_trie::LoudsTrie;
use std::fmt;
use std::io::Cursor;

/// Error returned when a dictionary buffer is truncated or structurally invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryError {
    /// The buffer ended before the field `field` could be read.
    UnexpectedEof { field: &'static str },
    /// The buffer was read completely but its contents are inconsistent.
    Invalid(&'static str),
}

impl fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DictionaryError::UnexpectedEof { field } => {
                write!(f, "invalid dictionary: unexpected end of data while reading {}", field)
            }
            DictionaryError::Invalid(reason) => write!(f, "invalid dictionary: {}", reason),
        }
    }
}

impl std::error::Error for DictionaryError {}

/// Bounds-checked big-endian reader. Every length prefix is checked against the
/// remaining input *before* allocating, so a malicious header cannot trigger a
/// multi-gigabyte allocation.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize, field: &'static str) -> Result<&'a [u8], DictionaryError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|&end| end <= self.buf.len())
            .ok_or(DictionaryError::UnexpectedEof { field })?;
        let bytes = &self.buf[self.pos..end];
        self.pos = end;
        Ok(bytes)
    }

    fn u32(&mut self, field: &'static str) -> Result<u32, DictionaryError> {
        Ok(u32::from_be_bytes(self.take(4, field)?.try_into().unwrap()))
    }

    /// Reads a `u32` element count and checks that `count * elem_size` bytes remain.
    fn count(&mut self, elem_size: usize, field: &'static str) -> Result<usize, DictionaryError> {
        let count = self.u32(field)? as usize;
        let needed = count
            .checked_mul(elem_size)
            .ok_or(DictionaryError::UnexpectedEof { field })?;
        if needed > self.buf.len() - self.pos {
            return Err(DictionaryError::UnexpectedEof { field });
        }
        Ok(count)
    }

    fn bit_vector(&mut self, field: &'static str) -> Result<BitVector, DictionaryError> {
        let size_in_bits = self.u32(field)? as usize;
        let num_words = size_in_bits.div_ceil(64);
        let bytes = self.take(num_words.checked_mul(8).unwrap(), field)?;
        let words = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| u64::from_be_bytes(*c))
            .collect();
        Ok(BitVector::new(words, size_in_bits))
    }

    fn trie(&mut self, compact_hiragana: bool) -> Result<LoudsTrie, DictionaryError> {
        let edges: Vec<u16> = if compact_hiragana {
            let n = self.count(1, "key trie edges")?;
            self.take(n, "key trie edges")?
                .iter()
                .map(|&b| CompactDictionary::decode(b))
                .collect()
        } else {
            let n = self.count(2, "value trie edges")?;
            self.take(n * 2, "value trie edges")?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes(*c))
                .collect()
        };
        let bit_vector = self.bit_vector("trie bit vector")?;
        let trie = LoudsTrie { bit_vector, edges };
        trie.validate().map_err(DictionaryError::Invalid)?;
        Ok(trie)
    }
}

#[derive(Debug)]
pub struct CompactDictionary {
    key_trie: LoudsTrie,
    value_trie: LoudsTrie,
    mapping_bit_vector: BitVector,
    mapping: Vec<u32>,
    has_mapping_bit_list: BitList,
}

pub struct SearchIter<'a> {
    dict: &'a CompactDictionary,
    size: usize,
    offset: usize,
    value_start_pos: usize,
    i: usize,
}

impl<'a> Iterator for SearchIter<'a> {
    type Item = Vec<u16>;
    fn next(&mut self) -> Option<Vec<u16>> {
        if self.i < self.size {
            let _i = self.i;
            self.i = self.i + 1;
            return Some(
                self.dict
                    .value_trie
                    .get_key(self.dict.mapping[self.value_start_pos - (self.offset as usize) + _i] as usize),
            );
        } else {
            return None;
        }
    }
}

pub struct PredictiveSearchIter<'a> {
    dict: &'a CompactDictionary,
    // key_trieから前方一致で得られたノードIDのイテレータ
    key_node_indices: std::vec::IntoIter<usize>,
    // 現在のキーノードが持つ、値IDのリスト
    current_values: std::vec::IntoIter<u32>,
    // 値を取得する際に再利用するバッファ
    key_buffer: Vec<u16>,
}

impl<'a> Iterator for PredictiveSearchIter<'a> {
    type Item = Vec<u16>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(mapping_index) = self.current_values.next() {
                self.key_buffer.clear();
                self.dict
                    .value_trie
                    .get_key_into(mapping_index as usize, &mut self.key_buffer);
                return Some(self.key_buffer.clone());
            }

            let dict = self.dict;
            let next_values = self.key_node_indices.find_map(|node_index| {
                if dict.has_mapping(node_index) {
                    let value_start_pos = dict.mapping_bit_vector.select(node_index, false);
                    let value_end_pos = dict.mapping_bit_vector.next_clear_bit(value_start_pos + 1);
                    let size = value_end_pos - value_start_pos - 1;

                    if size > 0 {
                        let offset = dict.mapping_bit_vector.rank(value_start_pos, false);
                        let start = value_start_pos - offset;
                        let end = start + size;
                        return Some(dict.mapping[start..end].to_vec().into_iter());
                    }
                }
                None
            });

            if let Some(values) = next_values {
                self.current_values = values;
            } else {
                return None;
            }
        }
    }
}

impl CompactDictionary {
    /// Parses a dictionary, panicking if the data is malformed.
    ///
    /// Prefer [`CompactDictionary::try_new`] when the data comes from an untrusted source.
    pub fn new(buffer: &Vec<u8>) -> CompactDictionary {
        CompactDictionary::try_new(buffer).unwrap_or_else(|e| panic!("{}", e))
    }

    /// Parses and validates a dictionary without panicking on malformed input.
    pub fn try_new(buffer: &[u8]) -> Result<CompactDictionary, DictionaryError> {
        let mut reader = Reader { buf: buffer, pos: 0 };
        let key_trie = reader.trie(true)?;
        let value_trie = reader.trie(false)?;
        let mapping_bit_vector = reader.bit_vector("mapping bit vector")?;
        let mapping_size = reader.count(4, "mapping")?;
        let mapping: Vec<u32> = reader
            .take(mapping_size * 4, "mapping")?
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_be_bytes(*c))
            .collect();

        // Each 1-bit of the mapping bit vector refers to one mapping entry, and every
        // reachable mapping entry must name an existing value node. (Dictionaries in the
        // wild carry one trailing, unreachable entry, so `mapping` may be longer.)
        let reachable = mapping_bit_vector.count(true);
        if mapping.len() < reachable {
            return Err(DictionaryError::Invalid(
                "mapping is shorter than the mapping bit vector",
            ));
        }
        if mapping[..reachable]
            .iter()
            .any(|&v| v == 0 || v as usize >= value_trie.edges.len())
        {
            return Err(DictionaryError::Invalid("mapping refers to a nonexistent value"));
        }

        let has_mapping_bit_list = CompactDictionary::create_mapping_bit_list(&mapping_bit_vector);
        Ok(CompactDictionary {
            key_trie,
            value_trie,
            mapping_bit_vector,
            mapping,
            has_mapping_bit_list,
        })
    }

    /// Reads a trie from `cursor`, panicking if the data is malformed.
    pub fn read_trie(cursor: &mut Cursor<&Vec<u8>>, compact_hiragana: bool) -> LoudsTrie {
        let buf: &[u8] = cursor.get_ref();
        let mut reader = Reader {
            buf,
            pos: cursor.position() as usize,
        };
        let trie = reader.trie(compact_hiragana).unwrap_or_else(|e| panic!("{}", e));
        cursor.set_position(reader.pos as u64);
        trie
    }

    fn create_mapping_bit_list(bit_vector: &BitVector) -> BitList {
        let num_of_nodes = bit_vector.rank(bit_vector.size(), false);
        let mut bit_list = BitList::new_with_size(num_of_nodes + 1);
        let mut bit_position = 0;
        for node in 1..=num_of_nodes {
            let has_mapping = bit_position + 1 < bit_vector.size() && bit_vector.get(bit_position + 1);
            bit_list.set(node, has_mapping);
            bit_position = bit_vector.next_clear_bit(bit_position + 1)
        }
        return bit_list;
    }

    /// Whether key node `node_index` has at least one value. Nodes past the end of the
    /// mapping bit vector (e.g. the last key node) have none.
    fn has_mapping(&self, node_index: usize) -> bool {
        node_index < self.has_mapping_bit_list.len() && self.has_mapping_bit_list.get(node_index)
    }

    fn decode(b: u8) -> u16 {
        match b {
            // NULL文字
            0x00 => 0,
            // ASCIIの範囲 (0x20 ~ 0x7E)
            0x20..=0x7E => b as u16,
            // ひらがなの範囲 (0xA1 ~ 0xF6)
            0xA1..=0xF6 => {
                let code_point = (b as u32) - 0xA0 + 0x3040;
                code_point as u16
            }
            // 長音符 'ー'
            0xF7 => 0x30fc,
            // 未定義のバイト列が来た場合は 0 を返す
            _ => 0,
        }
    }

    pub fn search(&self, key: &Vec<u16>) -> SearchIter<'_> {
        let key_index = self.key_trie.get(key).filter(|&i| self.has_mapping(i));
        if key_index.is_some() {
            let key_index = key_index.unwrap();
            let value_start_pos = self.mapping_bit_vector.select(key_index as usize, false);
            let value_end_pos = self.mapping_bit_vector.next_clear_bit(value_start_pos + 1);
            let size = value_end_pos - value_start_pos - 1;
            if size > 0 {
                let offset = self.mapping_bit_vector.rank(value_start_pos, false);
                return SearchIter {
                    dict: self,
                    size: size as usize,
                    offset: offset as usize,
                    value_start_pos: value_start_pos,
                    i: 0,
                };
            }
        }
        return SearchIter {
            dict: self,
            offset: 0,
            size: 0,
            i: 0,
            value_start_pos: 0,
        };
    }

    pub fn predictive_search<'a>(&'a self, key: &[u16]) -> PredictiveSearchIter<'a> {
        // TODO: ノードIDのリストを取得してからイテレータを生成しているため、半遅延評価であり、効率が悪い
        let key_node_indices_vec = if let Some(key_index) = self.key_trie.get(key) {
            if key_index > 1 {
                self.key_trie.predictive_search(key_index).into_iter().collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        PredictiveSearchIter {
            dict: self,
            key_node_indices: key_node_indices_vec.into_iter(),
            current_values: Vec::new().into_iter(),
            key_buffer: Vec::with_capacity(16),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn test_1() {
        let mut f = File::open("todofuken").expect("Fail to load dict file");
        let mut buf = Vec::new();
        let _ = f.read_to_end(&mut buf);
        drop(f);
        let dict = CompactDictionary::new(&buf);
        let word: Vec<u16> = "おおさ".encode_utf16().collect();
        for s in dict.search(&word) {
            println!("{}", String::from_utf16_lossy(&s));
        }
    }

    fn todofuken() -> Vec<u8> {
        std::fs::read("todofuken").expect("Fail to load dict file")
    }

    #[test]
    fn try_new_rejects_truncated_data() {
        let buf = todofuken();
        for len in 0..buf.len() {
            assert!(CompactDictionary::try_new(&buf[..len]).is_err(), "len={}", len);
        }
        assert!(CompactDictionary::try_new(&buf).is_ok());
    }

    #[test]
    fn try_new_does_not_allocate_from_untrusted_lengths() {
        // A 4-byte header claiming ~2^31 edges used to abort with a 2 GiB allocation.
        let err = CompactDictionary::try_new(&[0x7f, 0xff, 0xff, 0xff]).unwrap_err();
        assert_eq!(
            err,
            DictionaryError::UnexpectedEof {
                field: "key trie edges"
            }
        );
    }

    #[test]
    fn query_on_last_trie_node_does_not_panic() {
        // "わかやまけん" is the last node of the key trie in BFS order.
        use crate::migemo::query::query;
        use crate::migemo::regex_generator::RegexOperator;
        let dict = CompactDictionary::try_new(&todofuken()).unwrap();
        for word in ["わかやまけん", "わかやまけんあ", "wakayamaken"] {
            assert!(!query(word.to_string(), &dict, &RegexOperator::Default).is_empty());
        }
        let key: Vec<u16> = "わかやまけん".encode_utf16().collect();
        let _ = dict.search(&key).count();
    }

    #[test]
    fn corrupted_dictionary_never_panics() {
        use crate::migemo::query::query;
        use crate::migemo::regex_generator::RegexOperator;
        let buf = todofuken();
        for i in 0..buf.len() {
            for x in [0x01u8, 0x80, 0xff] {
                let mut corrupted = buf.clone();
                corrupted[i] ^= x;
                if let Ok(dict) = CompactDictionary::try_new(&corrupted) {
                    for word in ["o", "ky", "wakayamaken"] {
                        let _ = query(word.to_string(), &dict, &RegexOperator::Default);
                    }
                }
            }
        }
    }
}
