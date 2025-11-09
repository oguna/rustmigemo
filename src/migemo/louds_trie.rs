use super::bit_vector::BitVector;

#[derive(Debug)]
pub struct LoudsTriePredictiveSearchIter<'a> {
    trie: &'a LoudsTrie,
    upper: usize,
    lower: usize,
    cursor: usize,
}

impl<'a> Iterator for LoudsTriePredictiveSearchIter<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.cursor < self.upper {
            let last_cursor = self.cursor;
            self.cursor = self.cursor + 1;
            return Some(last_cursor);
        } else if self.upper == self.lower {
            return None;
        } else {
            let lower_select = self.trie.bit_vector.select(self.lower, false);
            let upper_select = self.trie.bit_vector.select(self.upper, false);
            
            if lower_select + 1 <= self.trie.bit_vector.size() {
                self.lower = self.trie.bit_vector.rank(lower_select + 1, true) + 1;
            } else {
                self.lower = self.trie.bit_vector.rank(self.trie.bit_vector.size(), true) + 1;
            }
            
            if upper_select + 1 <= self.trie.bit_vector.size() {
                self.upper = self.trie.bit_vector.rank(upper_select + 1, true) + 1;
            } else {
                self.upper = self.trie.bit_vector.rank(self.trie.bit_vector.size(), true) + 1;
            }
            
            self.cursor = self.lower;
            if self.lower == self.upper {
                return None
            } else {
                return Some(self.lower);
            }
        }
    }
}

#[derive(Debug)]
pub struct LoudsTrieCommonPrefixSearchIter<'a, 'b> {
    trie: &'a LoudsTrie,
    key: &'b [u16],
    node_index: usize,
    position: usize,
    finished: bool,
}

impl<'a, 'b> Iterator for LoudsTrieCommonPrefixSearchIter<'a, 'b> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        if self.position >= self.key.len() {
            self.finished = true;
            return None;
        }
        match self
            .trie
            .traverse(self.node_index as u32, self.key[self.position])
        {
            Some(next_node) => {
                self.node_index = next_node;
                self.position += 1;
                Some(self.node_index)
            }
            None => {
                self.finished = true;
                None
            }
        }
    }
}

#[derive(Debug)]
pub struct LoudsTrie {
    pub bit_vector: BitVector,
    pub edges: Vec<u16>,
}

impl LoudsTrie {
    pub fn get_key(&self, mut index: usize) -> Vec<u16> {
        assert!(index > 0 && index < self.edges.len(), "get_key: Index {} is out of bounds (1..{})", index, self.edges.len());
        let mut sb: Vec<u16> = Vec::new();
        while index > 1 {
            sb.push(self.edges[index]);
            index = self.parent(index);
        }
        sb.reverse();
        return sb;
    }

    pub fn get_key_into(&self, mut index: usize, target: &mut Vec<u16>) -> usize {
        assert!(index > 0 && index < self.edges.len(), "get_key_into: Index {} is out of bounds (1..{})", index, self.edges.len());
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

    pub fn get(&self, key: &[u16]) -> Option<usize> {
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

    pub fn predictive_search(&self, node: usize) -> LoudsTriePredictiveSearchIter<'_> {
        let lower = node;
        let upper = node + 1;
        return LoudsTriePredictiveSearchIter {
            trie: self,
            upper: upper,
            lower: lower,
            cursor: lower,
        };
    }

    pub fn common_prefix_search<'a, 'b>(
        &'a self,
        key: &'b [u16],
    ) -> LoudsTrieCommonPrefixSearchIter<'a, 'b> {
        LoudsTrieCommonPrefixSearchIter {
            trie: self,
            key,
            node_index: 1,
            position: 0,
            finished: false,
        }
    }

    pub fn size(&self) -> usize {
        return self.edges.len() - 2;
    }

    pub fn build(keys: &Vec<Vec<u16>>) -> (LoudsTrie, Vec<u32>) {
        let mut memo: Vec<i32> = vec![1; keys.len()];
        let mut offset = 0;
        let mut current_node: usize = 1;
        let mut edges = vec![0x20, 0x20];
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

        // Ensure child_sizes is large enough for current_node
        if child_sizes.len() <= current_node {
            child_sizes.resize(current_node + 1, 0);
        }

        let num_of_children = child_sizes[1..=current_node].iter().sum::<u32>();

        let num_of_nodes = current_node;
        let mut bit_vector_words =
            vec![0u64; ((num_of_children + num_of_nodes as u32 + 63 + 1) / 64) as usize];
        let mut bit_vector_index = 1;
        bit_vector_words[0] = 1;
        for i in 1..=current_node {
            bit_vector_index = bit_vector_index + 1;
            let child_size = child_sizes[i];
            for _ in 0..child_size {
                bit_vector_words[bit_vector_index >> 6] =
                    bit_vector_words[bit_vector_index >> 6] | (1u64 << (bit_vector_index & 63));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_1() {
        let words: Vec<Vec<u16>> = vec!["baby", "bad", "bank", "box", "dad", "dance"].iter().map(|x| x.encode_utf16().collect()).collect();
        let (trie, x) = LoudsTrie::build(&words);
        println!("{:?}", x);
        let box_utf16: Vec<u16> = "box".encode_utf16().collect();
        let actual = trie.get(&box_utf16);
        assert_eq!(actual, Some(10));
        assert_eq!(trie.bit_vector.words(), vec![1145789805]);
        assert_eq!(trie.bit_vector.size(), 32);
        assert_eq!(trie.edges, vec![32, 32, 98, 100, 97, 111, 97, 98, 100, 110, 120, 100, 110, 121,107, 99, 101]);
    }

    #[test]
    fn test_common_prefix_search() {
        let words: Vec<Vec<u16>> = vec!["a", "ab", "abc", "abcd"].iter().map(|x| x.encode_utf16().collect()).collect();
        let (trie, _) = LoudsTrie::build(&words);
        
        let query: Vec<u16> = "abcd".encode_utf16().collect();
        let result: Vec<_> = trie.common_prefix_search(&query).collect();
        
        // Should find all prefixes: "a", "ab", "abc", "abcd"
        assert_eq!(result.len(), 4);
        
        // Verify each result is a valid node
        for node_index in result {
            assert!(node_index > 0 && node_index < trie.edges.len());
        }
    }

    #[test]
    fn test_common_prefix_search_partial() {
        let words: Vec<Vec<u16>> = vec!["baby", "bad", "bank"].iter().map(|x| x.encode_utf16().collect()).collect();
        let (trie, _) = LoudsTrie::build(&words);
        
        let query: Vec<u16> = "ba".encode_utf16().collect();
        let result: Vec<_> = trie.common_prefix_search(&query).collect();
        
        // Should find "b" and "ba" positions in the trie
        assert_eq!(result.len(), 2);
    }
}
