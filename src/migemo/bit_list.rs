
#[derive(Debug)]
pub struct BitList {
    pub words: Vec<u64>,
    pub size: usize,
}

impl BitList {
    pub fn new() -> BitList {
        return BitList {
            words: vec![0, 8],
            size: 0,
        }
    }

    pub fn new_with_size(size: usize) -> BitList {
        return BitList {
            words: vec![0; (size+63)/64],
            size: size,
        }
    }

    pub fn add(&mut self, value: bool) {
        if self.words.len() < (self.size + 64)/64 {
            self.words.push(0);
        }
        self.set(self.size, value);
        self.size = self.size + 1;
    }

    pub fn set(&mut self, pos: usize, value: bool) {
        if self.size < pos {
            panic!("index out of range [{}] with length {}", pos, self.size);
        }
        if value {
            self.words[pos/64] |= 1 << (pos % 64);
        } else {
            self.words[pos/64] &= !(1 << (pos % 64));
        }
    }

    pub fn get(&self, pos: usize) -> bool {
        if self.size < pos {
            panic!();
        }
        return (self.words[pos/64] >> (pos%64)) & 1 == 1;
    }
}