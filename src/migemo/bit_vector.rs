#[derive(Debug)]
pub struct BitVector {
    pub words: Vec<u64>,
    pub size_in_bits: usize,
    pub lb: Vec<u32>,
    pub sb: Vec<u16>,
}

impl BitVector {
    pub fn new(words: Vec<u64>, size_in_bits: usize) -> BitVector {
        if (size_in_bits + 63) / 64 != words.len() {
            panic!()
        }
        let mut lb: Vec<u32> = vec![0; (size_in_bits + 511) / 512];
        let mut sb: Vec<u16> = vec![0; lb.len() * 8];
        let mut sum: u32 = 0;
        let mut sum_in_lb: u32 = 0;
        for i in 0..sb.len() {
            let bit_count = if i < words.len() {
                u64::count_ones(words[i])
            } else {
                0
            };
            sb[i] = sum_in_lb as u16;
            sum_in_lb = sum_in_lb + bit_count;
            if (i & 7) == 7 {
                lb[i >> 3] = sum;
                sum = sum + sum_in_lb as u32;
                sum_in_lb = 0;
            }
        }
        return BitVector {
            words: words,
            size_in_bits: size_in_bits,
            lb: lb,
            sb: sb,
        }
    }

    pub fn rank(&self, pos: usize, b: bool) -> usize {
        if self.size_in_bits <= pos {
            //panic!();
        }
        let mut count1 = self.sb[(pos / 64) as usize] as usize + self.lb[(pos / 512) as usize] as usize;
        let word = self.words[(pos / 64) as usize];
        let shift_size = 64 - (pos & 63);
        let mask = if shift_size == 64 { 0 } else {0xFFFFFFFFFFFFFFFFu64 >> shift_size};
        count1 = count1 + u64::count_ones(word & mask) as usize;
        return if b {
            count1
        } else {
            pos - count1
        };
    }

    pub fn select(&self, count: usize, b: bool) -> usize {
        let lb_index = self.lower_bound_binary_search_lb(count as u32, b) - 1;
        let count_in_lb: usize = if b {
            count - self.lb[lb_index as usize] as usize
        } else {
            count - (512 * lb_index - self.lb[lb_index as usize] as usize) as usize
        };
        let sb_index = self.lower_bound_binary_search_sb(count_in_lb as u16, lb_index as usize * 8, lb_index as usize * 8 + 8, b) - 1;
        let count_in_sb = if b {
            count_in_lb - self.sb[sb_index] as usize
        } else {
            count_in_lb - (64 * (sb_index % 8) - self.sb[sb_index as usize] as usize)
        };
        let mut word = self.words[sb_index];
        if !b {
            word = !word;
        }
        return sb_index * 64 + BitVector::select_in_word(word, count_in_sb);
    }

    fn select_in_word(mut word: u64, mut count: usize) -> usize {
        let lower_bit_count = u32::count_ones(word as u32) as usize;
        let mut i = 0;
        if lower_bit_count < count {
            word = word >> 32;
            count = count - lower_bit_count;
            i = 32;
        }
        let lower16bit_count = u16::count_ones(word as u16) as usize;
        if lower16bit_count < count {
            word = word >> 16;
            count = count - lower16bit_count;
            i = i + 16;
        }
        let lower8bit_count = u8::count_ones(word as u8) as usize;
        if lower8bit_count < count {
            word = word >> 8;
            count = count - lower8bit_count;
            i = i + 8;
        }
        let lower4bit_count = u8::count_ones((word & 0b1111) as u8) as usize;
        if lower4bit_count < count {
            word = word >> 4;
            count = count - lower4bit_count;
            i = i + 4;
        }
        while count > 0 {
            count = count - (word & 1) as usize;
            word = word >> 1;
            i = i + 1;
        }
        return i - 1;
    }
    
    fn lower_bound_binary_search_lb(&self, key: u32, b: bool) -> usize {
        let mut high = self.lb.len() as isize;
        let mut low: isize = -1;
        if b {
            while high - low > 1 {
                let mid = (high + low) / 2;
                if self.lb[mid as usize] < key {
                    low = mid;
                } else {
                    high = mid;
                }
            }
        } else {
            while high - low > 1 {
                let mid = (high + low) / 2;
                if ((mid << 9) as u32) - self.lb[mid as usize] < key {
                    low = mid
                } else {
                    high = mid
                }
            }
        }
        return high as usize;
    }

    fn lower_bound_binary_search_sb(&self, key: u16, from_index: usize, to_index: usize, b: bool) -> usize {
        let mut high = to_index as isize;
        let mut low = from_index as isize - 1;
        if b {
            while high-low > 1 {
                let mid = (high + low) / 2;
                if self.sb[mid as usize] < key {
                    low = mid;
                } else {
                    high = mid;
                }
            }
        } else {
            while high-low > 1 {
                let mid = (high + low) >> 1;
                if (((mid&7) <<6) as u16)-self.sb[mid as usize] < key {
                    low = mid;
                } else {
                    high = mid;
                }
            }
        }
        return high as usize;
    }

    pub fn next_clear_bit(&self, from_index: usize) -> usize {
        let mut u = from_index >> 6;
        if u >= self.words.len() {
            return from_index;
        }
        let mut word = !self.words[u] & (0xFFFFFFFFFFFFFFFFu64 << (from_index & 63));
        loop {
            if word != 0 {
                return (u * 64) + (word.trailing_zeros() as usize);
            }
            u = u + 1;
            if u == self.words.len() {
                return 64 * self.words.len();
            }
            word = !self.words[u];
        }
    }

    pub fn size(&self) -> usize {
        return self.size_in_bits as usize;
    }

    pub fn get(&self, pos: usize) -> bool {
        if self.size_in_bits < pos {
            panic!();
        }
        return ((self.words[(pos >> 6) as usize] >> (pos & 63)) & 1) == 1;
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;

    fn bits_to_words(bits: &Vec<bool>) -> Vec<u64> {
        let mut words = vec![0; (bits.len()+63)/64];
        for i in 0..bits.len() {
            if bits[i] {
                words[i/64] |= 1 << (i & 63);
            }
        }
        return words;
    }

    #[test]
    fn test_rank() {
        fn rank(bits: &Vec<bool>, pos: usize, b: bool) -> usize {
            let mut count = 0;
            for i in 0..pos {
                if bits[i] == b {
                    count = count + 1;
                }
            }
            return count;
        }
        const SIZE: usize = 10000;
        let mut rng = rand::rng();
        let mut bits: Vec<bool> = vec![false; SIZE];
        for i in 0..bits.len() {
            bits[i] = rng.random();
        }
        let words = bits_to_words(&bits);
        let bitvector = BitVector::new(words, SIZE);
        for i in 0..SIZE {
            assert_eq!(rank(&bits, i, true), bitvector.rank(i, true) as usize);
            assert_eq!(rank(&bits, i, false), bitvector.rank(i, false) as usize);
        }
    }

    #[test]
    fn test_select() {
        fn select(bits: &Vec<bool>, count: usize, b: bool) -> Result<usize, &str> {
            let mut count = count;
            for i in 0..bits.len() {
                if bits[i] == b {
                    count = count - 1;
                }
                if count == 0 {
                    return Ok(i);
                }
            }
            return Err("");
        }
        const SIZE: usize = 10000;
        let mut rng = rand::rng();
        let mut bits: Vec<bool> = vec![false; SIZE];
        for i in 0..bits.len() {
            bits[i] = rng.random();
        }
        let words = bits_to_words(&bits);
        let mut count1 = 0;
        for i in 0..bits.len() {
            if bits[i] {
                count1 = count1 + 1;
            }
        }
        let bitvector = BitVector::new(words, SIZE);
        let count0 = bits.len() - count1;
        for i in 1..count1 {
            assert_eq!(select(&bits, i ,true).unwrap(), bitvector.select(i, true));
        }
        for i in 1..count0 {
            assert_eq!(select(&bits, i , false).unwrap(), bitvector.select(i, false));
        }
    }

    #[test]
    fn test_next_clear_bit() {
        fn next_clear_bit(bits: &Vec<bool>, pos: usize) -> usize {
            let mut pos = pos;
            while pos < bits.len() && bits[pos] {
                pos = pos + 1;
            }
            if pos > bits.len() {
                return bits.len() + 1;
            } else {
                return pos;
            }
        }
        const SIZE: usize = 1000;
        let mut rng = rand::rng();
        let mut bits: Vec<bool> = vec![false; SIZE];
        for i in 0..bits.len() {
            bits[i] = rng.random();
        }
        let words = bits_to_words(&bits);
        let bitvector = BitVector::new(words, SIZE);
        for i in 0..=SIZE {
            assert_eq!(next_clear_bit(&bits, i), bitvector.next_clear_bit(i) as usize);
        }
    }
}