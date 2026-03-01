#[derive(Debug)]
pub struct BitList {
    words: Vec<u64>,
    size: usize,
}

impl Default for BitList {
    fn default() -> Self {
        Self::new()
    }
}

impl BitList {
    pub fn new() -> BitList {
        BitList {
            words: Vec::with_capacity(8),
            size: 0,
        }
    }

    pub fn new_with_size(size: usize) -> BitList {
        BitList {
            words: vec![0; (size + 63) / 64],
            size: size,
        }
    }

    pub fn push(&mut self, value: bool) {
        let word_idx = self.size / 64;
        let bit_idx = self.size % 64;
        if word_idx >= self.words.len() {
            self.words.push(0);
        }
        if value {
            self.words[word_idx] |= 1 << bit_idx;
        } else {
            self.words[word_idx] &= !(1 << bit_idx);
        }
        self.size += 1;
    }

    pub fn set(&mut self, pos: usize, value: bool) {
        if self.size <= pos {
            panic!("index out of bounds: the len is {} but the index is {}", self.size, pos);
        }
        if value {
            self.words[pos / 64] |= 1 << (pos % 64);
        } else {
            self.words[pos / 64] &= !(1 << (pos % 64));
        }
    }

    pub fn get(&self, pos: usize) -> bool {
        if self.size <= pos {
            panic!("index out of bounds: the len is {} but the index is {}", self.size, pos);
        }
        (self.words[pos / 64] >> (pos % 64)) & 1 == 1
    }

    pub fn words(&self) -> &[u64] {
        &self.words
    }

    pub fn len(&self) -> usize {
        self.size
    }
}
