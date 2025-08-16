use super::bit_vector::BitVector;
use super::bit_list::BitList;
use super::louds_trie::LoudsTrie;
use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;

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
            return Some(self.dict.value_trie.get_key(
                self.dict.mapping[self.value_start_pos - (self.offset as usize) + _i] as usize,
            ));
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
                if node_index < dict.has_mapping_bit_list.len() && dict.has_mapping_bit_list.get(node_index) {
                    let value_start_pos = dict.mapping_bit_vector.select(node_index, false);
                    let value_end_pos = dict
                        .mapping_bit_vector
                        .next_clear_bit(value_start_pos + 1);
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
    pub fn new(buffer: &Vec<u8>) -> CompactDictionary {
        let mut cursor = Cursor::new(buffer);
        let key_trie = CompactDictionary::read_trie(&mut cursor, true);
        let value_trie = CompactDictionary::read_trie(&mut cursor, false);
        let mapping_bit_vector_size = cursor.read_u32::<BigEndian>().unwrap() as usize;
        let mut mapping_bit_vector_words =
            vec![0; (mapping_bit_vector_size + 63) / 64];
        for i in 0..mapping_bit_vector_words.len() {
            mapping_bit_vector_words[i] = cursor.read_u64::<BigEndian>().unwrap();
        }
        let mapping_bit_vector =
            BitVector::new(mapping_bit_vector_words, mapping_bit_vector_size);
        let mapping_size = cursor.read_u32::<BigEndian>().unwrap();
        let mut mapping: Vec<u32> = vec![0; mapping_size as usize];
        for i in 0..mapping_size {
            mapping[i as usize] = cursor.read_u32::<BigEndian>().unwrap();
        }
        let has_mapping_bit_list = CompactDictionary::create_mapping_bit_list(&mapping_bit_vector);
        return CompactDictionary {
            key_trie: key_trie,
            value_trie: value_trie,
            mapping_bit_vector: mapping_bit_vector,
            mapping: mapping,
            has_mapping_bit_list: has_mapping_bit_list,
        };
    }

    pub fn read_trie(cursor: &mut Cursor<&Vec<u8>>, compact_hiragana: bool) -> LoudsTrie {
        let key_trie_edge_size = cursor.read_u32::<BigEndian>().unwrap();
        let mut key_trie_edges = vec![0; key_trie_edge_size as usize];
        for i in 0..key_trie_edge_size {
            let c: u16 = if compact_hiragana {
                CompactDictionary::decode(cursor.read_u8().unwrap())
            } else {
                cursor.read_u16::<BigEndian>().unwrap()
            };
            key_trie_edges[i as usize] = c;
        }
        let key_trie_bit_vector_size = cursor.read_u32::<BigEndian>().unwrap();
        let mut key_trie_bit_vector_words: Vec<u64> =
            vec![0; (key_trie_bit_vector_size as usize + 63) / 64];
        for i in 0..key_trie_bit_vector_words.len() {
            key_trie_bit_vector_words[i] = cursor.read_u64::<BigEndian>().unwrap();
        }
        let bit_vector =
            BitVector::new(key_trie_bit_vector_words, key_trie_bit_vector_size as usize);
        let louds_trie = LoudsTrie {
            bit_vector: bit_vector,
            edges: key_trie_edges,
        };
        return louds_trie;
    }

    fn create_mapping_bit_list(bit_vector: &BitVector) -> BitList {
        let num_of_nodes = bit_vector.rank(bit_vector.size(), false);
        let mut bit_list = BitList::new_with_size(num_of_nodes + 1);
        let mut bit_position = 0;
        for node in 1..=num_of_nodes {
            let has_mapping = bit_vector.get(bit_position + 1);
            bit_list.set(node, has_mapping);
            bit_position = bit_vector.next_clear_bit(bit_position + 1)
        }
        return bit_list;
    }

    fn decode(c: u8) -> u16 {
        if 0x20 <= c && c <= 0x7e {
            return c as u16;
        }
        if 0xa1 <= c && c <= 0xf6 {
            return (c as u16) + 0x3040 - 0xa0;
        }   
        return 0;
    }

    pub fn search(&self, key: &Vec<u16>) -> SearchIter<'_> {
        let key_index = self.key_trie.get(key);
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
}
