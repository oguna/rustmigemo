use super::bit_vector::BitVector;
#[derive(Debug)]
pub struct LoudsTrie {
    pub bit_vector: BitVector,
    pub edges: Vec<u16>,
}

impl LoudsTrie {
    pub fn get_key(&self, mut index: usize) -> Vec<u16> {
        if index <= 0 || self.edges.len() <= index {
            panic!();
        }
        let mut sb: Vec<u16> = Vec::new();
        while index > 1 {
            sb.push(self.edges[index]);
            index = self.parent(index);
        }
        sb.reverse();
        return sb;
    }

    pub fn get_key2(&self, mut index: usize, target: &mut Vec<u16>) -> usize {
        if index <= 0 || self.edges.len() <= index {
            panic!();
        }
        target.clear();
        while index > 1 {
            target.push(self.edges[index]);
            index = self.parent(index);
        }
        target.reverse();
        return target.len();
    }

    pub fn parent(&self, x: usize) -> usize {
        return self
            .bit_vector
            .rank(self.bit_vector.select(x, true), false);
    }

    pub fn first_child(&self, x: usize) -> Option<usize> {
        let y = self.bit_vector.select(x, false) + 1;
        if self.bit_vector.get(y) {
            return Some(self.bit_vector.rank(y, true) + 1);
        } else {
            return None;
        }
    }

    pub fn traverse(&self, index: u32, c: u16) -> Option<usize> {
        let first_child = self.first_child(index as usize);
        if first_child.is_none() {
            return None;
        }
        let first_child = first_child.unwrap();
        let child_start_bit = self.bit_vector.select(first_child as usize, true);
        let child_end_bit = self.bit_vector.next_clear_bit(child_start_bit);
        let child_size = child_end_bit - child_start_bit;
        let result = LoudsTrie::binary_search_uint16(&self.edges, first_child as usize, (first_child as usize) + child_size, c);
        return match result {
            Ok(x) => Some(x),
            Err(_) => None
        }
    }

    pub fn get(&self, key: &Vec<u16>) -> Option<usize> {
        let mut node_index = 1;
        for c in key {
            let result = self.traverse(node_index as u32, *c);
            if result.is_some() {
                node_index = result.unwrap();
            } else {
                return None;
            }
        }
        return Some(node_index);
    }

    pub fn iterator(&self, index: u32) -> Vec<u32> {
        let mut result: Vec<u32> = Vec::new();
        result.push(index);
        let child = self.first_child(index as usize);
        if child.is_none() {
            return result;
        }
        let mut child = child.unwrap();
        let mut child_pos = self.bit_vector.select(child as usize, true);
        while self.bit_vector.get(child_pos) {
            result.extend(self.iterator(child as u32));
            child = child + 1;
            child_pos = child_pos + 1;
        }
        return result;
    }

    pub fn iterator2(&self, index: u32, result: &mut Vec<u32>) -> usize {
        let mut count = 0;
        result.push(index);
        let child = self.first_child(index as usize);
        if child.is_none() {
            return 1;
        }
        let mut child = child.unwrap();
        let mut child_pos = self.bit_vector.select(child as usize, true);
        while self.bit_vector.get(child_pos) {
            count = count + self.iterator2(child as u32, result);
            child = child + 1;
            child_pos = child_pos + 1;
        }
        return count;
    }

    pub fn size(&self) -> usize {
        return self.edges.len() - 2;
    }

    pub fn build(keys: &Vec<Vec<u16>>) -> (LoudsTrie, Vec<u32>) {
        let mut memo: Vec<i32> = vec![1; keys.len()];
        let mut offset = 0;
        let mut current_node: usize = 1;
        let mut edges = vec![0x30, 0x30];
        let mut child_sizes: Vec<u32> = vec![0; 128];
        loop {
            let mut last_char = 0;
            let mut last_parent = 0;
            let mut rest_keys = 0;
            for i in 0..keys.len() {
                if memo[i] < 0 {
                    continue;
                }
                if keys[i].len() <= offset {
                    memo[i] = -memo[i];
                    continue;
                }
                let current_char = keys[i][offset];
                let current_parent = memo[i];
                if last_char != current_char || last_parent != current_parent {
                    if child_sizes.len() <= memo[i] as usize {
                        child_sizes.resize(child_sizes.len() * 2, 0);
                    }
                    child_sizes[memo[i] as usize] = child_sizes[memo[i] as usize] + 1;
                    current_node = current_node + 1;
                    edges.push(current_char);
                    last_char = current_char;
                    last_parent = current_parent;
                }
                memo[i] = current_node as i32;
                rest_keys = rest_keys + 1;
            }
            if rest_keys == 0 {
                break;
            }
            offset = offset + 1;
        }
        for i in 0..memo.len() {
            memo[i] = -memo[i];
        }

        let num_of_children = child_sizes[1..=current_node].iter().sum::<u32>();

        let num_of_nodes = current_node;
        let mut bit_vector_words =
            vec![0; ((num_of_children + num_of_nodes as u32 + 63 + 1) / 64) as usize];
        let mut bit_vector_index = 1;
        bit_vector_words[0] = 1;
        for i in 1..=current_node {
            bit_vector_index = bit_vector_index + 1;
            let child_size = child_sizes[i];
            for _ in 0..child_size {
                bit_vector_words[bit_vector_index >> 5] =
                    bit_vector_words[bit_vector_index >> 5] | (1 << (bit_vector_index & 31));
                bit_vector_index = bit_vector_index + 1;
            }
        }
        let bit_vector = BitVector::new(bit_vector_words, bit_vector_index);
        let louds_trie = LoudsTrie {
            bit_vector: bit_vector,
            edges: edges,
        };
        let generated_indexes = memo.iter().map(|x| *x as u32).collect();
        return (louds_trie, generated_indexes);
    }

    fn binary_search_uint16(a: &Vec<u16>, from: usize, to: usize, key: u16) -> Result<usize, usize> {
        // TODO: slice has binary_search, so we should use it, alternative to this implementation.
        let mut low = from;
        let mut high = to - 1;
        while low <= high {
            let mid = (low + high) >> 1;
            let mid_val = a[mid];
            if mid_val < key {
                low = mid + 1;
            } else if mid_val > key {
                high = mid - 1;
            } else {
                return Ok(mid);
            }
        }
        return Err(low + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_1() {
        let words: Vec<Vec<u16>> = vec!["baby", "bad", "bank", "box", "dad", "dance"].iter().map(|x| x.encode_utf16().collect()).collect();
        let (trie, x) = LoudsTrie::build(&words);
        println!("{:?}", x);
        let actual = trie.get(&"box".encode_utf16().collect());
        assert_eq!(actual, Some(10));
        assert_eq!(trie.bit_vector.words, vec![1145789805]);
        assert_eq!(trie.bit_vector.size_in_bits, 32);
        assert_eq!(trie.edges, vec![48, 48, 98, 100, 97, 111, 97, 98, 100, 110, 120, 100, 110, 121,107, 99, 101]);
    }
}
