use std::collections::HashMap;

use byteorder::{BigEndian, WriteBytesExt};

use super::{bit_list::BitList, louds_trie::LoudsTrie};

fn encode_char(c: char) -> Option<u16> {
    if c == '\u{00}' {
        return Some(0);
    }
    if '\u{20}' <= c && c <= '\u{7e}' {
        return Some(c as u16);
    }
    if '\u{3041}' <= c && c <= '\u{3096}' {
        return Some((c as u16) - 0x3040 + 0xa0);
    }
    if '\u{30fc}' == c {
        return Some((c as u16) - 0x3040 + 0xa0);
    }
    return None;
}

pub fn build(mut dict: HashMap<String, Vec<String>>) -> Vec<u8> {
    // remove some keys
    let mut keys_to_remove = Vec::new();
    for key in dict.keys() {
        for c in key.chars() {
            let encoded = encode_char(c);
            match encoded {
                Some(_) => {}
                None => {
                    keys_to_remove.push(key.clone());
                    println!("skipped the word: {}", key);
                }
            }
        }
    }
    for key in keys_to_remove {
        dict.remove(&key);
    }

    // build key trie
    let mut keys: Vec<Vec<u16>> = dict.keys().map(|s| s.encode_utf16().collect()).collect();
    keys.sort();
    let key_trie = LoudsTrie::build(&keys).0;

    // build value trie
    let mut values_set = std::collections::HashSet::new();
    for value in dict.values() {
        for v in value {
            values_set.insert(v.clone());
        }
    }
    let mut values: Vec<Vec<u16>> = values_set.iter().map(|s| s.encode_utf16().collect()).collect();
    values.sort();
    let value_trie = LoudsTrie::build(&values).0;

    // build trie mapping
    let mut mapping_count = 0;
    for i in dict.values() {
        mapping_count += i.len();
    }
    let mut mapping: Vec<u32> = vec![0; mapping_count];
    let mut mapping_index = 0;
    let mut mapping_bit_list = BitList::new();
    for i in 1..=key_trie.size() + 1 {
        let key = key_trie.get_key(i);
        mapping_bit_list.add(false);
        if let Some(values) = dict.get(&String::from_utf16_lossy(&key)) {
            for j in 0..values.len() {
                mapping_bit_list.add(true);
                let a: Vec<u16> = values[j].encode_utf16().collect();
                mapping[mapping_index] = value_trie.get(&a).unwrap() as u32;
                mapping_index += 1;
            }
        }
    }

    // calculate output size
    let key_trie_data_size =
        8 + key_trie.edges.len() + ((key_trie.bit_vector.size() + 63) >> 6) * 8;
    let value_trie_data_size =
        8 + value_trie.edges.len() * 2 + ((value_trie.bit_vector.size() + 63) >> 6) * 8;
    let mapping_data_size = 8 + ((mapping_bit_list.size + 63) >> 6) * 8 + mapping.len() * 4;
    let output_data_size = key_trie_data_size + value_trie_data_size + mapping_data_size;

    // ready output
    let mut output_data: Vec<u8> = Vec::with_capacity(output_data_size);

    // output key trie
    output_data.write_i32::<BigEndian>(key_trie.edges.len() as i32).unwrap();
    for edge in key_trie.edges {
        let compact_char = encode_char(char::from_u32(edge as u32).unwrap()).unwrap();
        output_data.write_u8(compact_char as u8).unwrap();
    }
    output_data.write_i32::<BigEndian>(key_trie.bit_vector.size() as i32).unwrap();
    for word in key_trie.bit_vector.words {
        output_data.write_u64::<BigEndian>(word).unwrap();
    }

    // output value trie
    output_data.write_i32::<BigEndian>(value_trie.edges.len() as i32).unwrap();
    for edge in value_trie.edges {
        output_data.write_u16::<BigEndian>(edge).unwrap();
    }
    output_data.write_i32::<BigEndian>(value_trie.bit_vector.size() as i32).unwrap();
    for word in value_trie.bit_vector.words {
        output_data.write_u64::<BigEndian>(word).unwrap();
    }

    // output mapping
    output_data.write_i32::<BigEndian>(mapping_bit_list.size as i32).unwrap();
    let mapping_words_len = (mapping_bit_list.size + 63) >> 6;
    for i in 0..mapping_words_len {
        output_data.write_u64::<BigEndian>(mapping_bit_list.words[i]).unwrap();
    }
    output_data.write_i32::<BigEndian>(mapping.len() as i32).unwrap();
    for value in mapping {
        output_data.write_u32::<BigEndian>(value).unwrap();
    }

    // check data size
    let data_view_index = output_data.len();
    if data_view_index != output_data_size {
        panic!("file size is not valid: expected={}, actual={}", output_data_size, data_view_index);
    }
    return output_data;
}

mod tests {

	#[test]
	fn test_1() {
        use std::collections::HashMap;
        use crate::migemo::compact_dictionary::CompactDictionary;
        use super::build;
        let mut dict = HashMap::new();
        dict.insert("けんさ".to_string(), vec!["検査".to_string()]);
        dict.insert("けんさく".to_string(), vec!["検索".to_string(),"研削".to_string()]);
        let buffer = build(dict);
        let compact_dict = CompactDictionary::new(&buffer);
        let a: Vec<u16> = "けんさく".encode_utf16().collect();
        let mut result: Vec<String> = Vec::new();
        for s in compact_dict.search(&a) {
            result.push(String::from_utf16(&s).unwrap());
        }
        assert_eq!(result[0], "検索");
        assert_eq!(result[1], "研削");
        assert_eq!(result.len(), 2);

        let expected_buffer: Vec<u8> = vec![
            0x00, 0x00, 0x00, 0x06, 0x20, 0x20, 0xB1, 0xF3, 0xB5, 0xAF, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x55, 0x00, 0x00, 0x00, 0x07, 0x00, 0x20, 0x00, 0x20, 0x69, 0x1C,
            0x78, 0x14, 0x67, 0xFB, 0x7D, 0x22, 0x52, 0x4A, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x6D, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xD0, 
            0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x06
        ];
        assert_eq!(buffer, expected_buffer);
	}
}
