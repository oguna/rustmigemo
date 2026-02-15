#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::_pdep_u64;

const SELECT8: [[u8; 8]; 256] = build_select8();

const fn build_select8() -> [[u8; 8]; 256] {
    let mut table = [[0xFFu8; 8]; 256];
    let mut byte = 0usize;
    while byte < 256 {
        let mut count = 0usize;
        let mut bit = 0usize;
        while bit < 8 {
            if ((byte as u8 >> bit) & 1) == 1 {
                table[byte][count] = bit as u8;
                count += 1;
            }
            bit += 1;
        }
        byte += 1;
    }
    table
}

#[derive(Debug)]
pub struct BitVector {
    words: Vec<u64>,
    size_in_bits: usize,
    lb: Vec<u32>,
    sb: Vec<u16>,
}

impl BitVector {
    pub fn new(words: Vec<u64>, size_in_bits: usize) -> BitVector {
        assert!(
            (size_in_bits + 63) / 64 == words.len(),
            "Word vector length does not match the size in bits. Expected {}, got {}.",
            (size_in_bits + 63) / 64,
            words.len()
        );
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
        BitVector {
            words: words,
            size_in_bits: size_in_bits,
            lb: lb,
            sb: sb,
        }
    }

    pub fn rank(&self, pos: usize, b: bool) -> usize {
        assert!(pos <= self.size_in_bits, "pos is out of bounds for rank");
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
        assert!(count > 0, "select() requires a 1-indexed count, but got 0.");
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
        sb_index * 64 + Self::select_in_word(word, count_in_sb)
    }

    #[inline]
    fn select_in_word(word: u64, count: usize) -> usize {
        use std::sync::OnceLock;
        type SelectFn = fn(u64, usize) -> usize;
        static SELECT_FN: OnceLock<SelectFn> = OnceLock::new();

        let func = SELECT_FN.get_or_init(|| {
            #[cfg(target_arch = "x86_64")]
            {
                if is_x86_feature_detected!("bmi2") {
                    return Self::select_in_word_bmi2_dispatch
                }
            }
            Self::select_in_word_fallback
        });
        func(word, count)
    }

    #[cfg(target_arch = "x86_64")]
    #[inline]
    fn select_in_word_bmi2_dispatch(word: u64, count: usize) -> usize {
        unsafe { Self::select_in_word_pdep(word, count) }
    }

    #[cfg(target_arch = "x86_64")]
    #[inline]
    unsafe fn select_in_word_pdep(word: u64, count: usize) -> usize {
        let k_th_bit = 1_u64 << (count - 1);
        let isolated_bit = unsafe { _pdep_u64(k_th_bit, word) };
        isolated_bit.trailing_zeros() as usize
    }

    #[inline]
    fn select_in_word_fallback(word: u64, count: usize) -> usize {
        // count is 1-indexed.
        assert!(count > 0, "count must be greater than 0 for select_in_word");

        // Ensure the word has enough set bits.
        assert!(
            word.count_ones() as usize >= count,
            "word (popcount: {}) has fewer than the required {} bits",
            word.count_ones(),
            count
        );

        let k0 = (count - 1) as u64;
        let mut b = word;
        b = b - ((b >> 1) & 0x5555555555555555);
        b = (b & 0x3333333333333333) + ((b >> 2) & 0x3333333333333333);
        b = (b + (b >> 4)) & 0x0F0F0F0F0F0F0F0F;

        let mut ps = b;
        ps += ps << 8;
        ps += ps << 16;
        ps += ps << 32;

        let k_rep = k0 * 0x0101010101010101;
        let high = 0x8080808080808080u64;
        // Each byte in ps is 0..64 and k0 is 0..63, so the subtract is borrow-free per byte.
        let le_mask = ((k_rep | high).wrapping_sub(ps)) & high;
        let gt_mask = (!le_mask) & high;
        let byte_idx = (gt_mask.trailing_zeros() >> 3) as u32;

        let prev_ps = ps << 8;
        let prev = ((prev_ps >> (byte_idx * 8)) & 0xFF) as u32;
        let k_in = (k0 as u32) - prev;
        let byte = ((word >> (byte_idx * 8)) & 0xFF) as u8;
        let pos_in_byte = SELECT8[byte as usize][k_in as usize];
        (byte_idx as usize) * 8 + (pos_in_byte as usize)
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
        assert!(
            pos < self.size_in_bits,
            "get() access out of bounds: pos (is {}) must be less than size_in_bits (is {})",
            pos,
            self.size_in_bits
        );
        return ((self.words[(pos >> 6) as usize] >> (pos & 63)) & 1) == 1;
    }

    pub fn words(&self) -> &[u64] {
        &self.words
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

    fn select_in_word_ref(word: u64, count: usize) -> usize {
        let mut remaining = count;
        for i in 0..64 {
            if ((word >> i) & 1) != 0 {
                remaining -= 1;
                if remaining == 0 {
                    return i;
                }
            }
        }
        panic!("select_in_word_ref called with count larger than popcount");
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
    fn test_select_in_word_edge_cases() {
        let cases = [
            0u64,
            !0u64,
            1u64,
            1u64 << 63,
            0xAAAAAAAAAAAAAAAA,
            0x5555555555555555,
            0x00000000FFFFFFFF,
            0xFFFFFFFF00000000,
            0x0000FFFF00000000,
            0x000000000000FFFF,
            0xFFFF000000000000,
        ];

        for &word in &cases {
            let pop = word.count_ones() as usize;
            if pop == 0 {
                continue;
            }
            for count in 1..=pop {
                assert_eq!(select_in_word_ref(word, count), BitVector::select_in_word(word, count));
            }
        }
    }

    #[test]
    fn test_select_in_word_random() {
        let mut rng = rand::rng();
        for _ in 0..2000 {
            let word: u64 = rng.random();
            let pop = word.count_ones() as usize;
            for count in 1..=pop {
                assert_eq!(select_in_word_ref(word, count), BitVector::select_in_word(word, count));
            }
        }
    }

    #[test]
    fn test_select_in_word_pdep_matches_when_available() {
        if !is_x86_feature_detected!("bmi2") {
            return;
        }
        let mut rng = rand::rng();
        for _ in 0..1000 {
            let word: u64 = rng.random();
            let pop = word.count_ones() as usize;
            for count in 1..=pop {
                let pdep_pos = unsafe { BitVector::select_in_word_pdep(word, count) };
                assert_eq!(pdep_pos, BitVector::select_in_word(word, count));
            }
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

