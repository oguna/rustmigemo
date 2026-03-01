#[derive(Debug, Default)]
pub struct SortedArrayTrie {
    keys: Vec<Vec<u16>>,
}

impl SortedArrayTrie {
    pub fn build(mut keys: Vec<Vec<u16>>) -> SortedArrayTrie {
        keys.sort();
        SortedArrayTrie { keys }
    }

    pub fn exact_search(&self, key: &[u16]) -> Option<usize> {
        self.keys
            .binary_search_by(|candidate| candidate.as_slice().cmp(key))
            .ok()
    }

    pub fn get(&self, index: usize) -> Option<&[u16]> {
        self.keys.get(index).map(Vec::as_slice)
    }

    pub fn common_prefix_search(&self, key: &[u16]) -> Vec<usize> {
        let mut result = Vec::new();
        let mut start = 0;
        let mut end = self.keys.len();

        for prefix_len in 1..=key.len() {
            let prefix = &key[..prefix_len];
            start += self.keys[start..end].partition_point(|candidate| candidate.as_slice() < prefix);
            end = start + self.keys[start..end].partition_point(|candidate| candidate.as_slice().starts_with(prefix));

            if start == end {
                break;
            }
            if self.keys[start].len() == prefix_len {
                result.push(start);
            }
        }
        result
    }

    pub fn predictive_search(&self, key: &[u16]) -> Vec<usize> {
        if key.is_empty() {
            return vec![];
        }

        let start = self.keys.partition_point(|candidate| candidate.as_slice() < key);
        let end = start + self.keys[start..].partition_point(|candidate| candidate.as_slice().starts_with(key));

        (start..end).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    fn decode_hits(dict: &SortedArrayTrie, hits: Vec<usize>) -> Vec<String> {
        hits.into_iter()
            .map(|index| String::from_utf16(dict.get(index).unwrap()).unwrap())
            .collect()
    }

    #[test]
    fn exact_search_returns_matching_index() {
        let dict = SortedArrayTrie::build(vec![utf16("けん"), utf16("けんさ"), utf16("けんさく")]);

        let word = String::from_utf16(dict.get(dict.exact_search(&utf16("けんさ")).unwrap()).unwrap()).unwrap();

        assert_eq!(word, "けんさ");
    }

    #[test]
    fn exact_search_returns_none_when_missing() {
        let dict = SortedArrayTrie::build(vec![utf16("けん"), utf16("けんさ"), utf16("けんさく")]);

        assert_eq!(dict.exact_search(&utf16("けんし")), None);
    }

    #[test]
    fn common_prefix_search_returns_registered_prefixes() {
        let dict = SortedArrayTrie::build(vec![
            utf16("け"),
            utf16("けん"),
            utf16("けんさ"),
            utf16("けんさく"),
            utf16("けんさくしゃ"),
            utf16("けい"),
        ]);

        let words = decode_hits(&dict, dict.common_prefix_search(&utf16("けんさくしゃたち")));

        assert_eq!(words, vec!["け", "けん", "けんさ", "けんさく", "けんさくしゃ"]);
    }

    #[test]
    fn common_prefix_search_returns_empty_when_no_prefix_is_registered() {
        let dict = SortedArrayTrie::build(vec![utf16("けん"), utf16("けんさ"), utf16("けんさく")]);

        let words = decode_hits(&dict, dict.common_prefix_search(&utf16("こ")));

        assert!(words.is_empty());
    }

    #[test]
    fn predictive_search_finds_all_prefix_matches() {
        let dict = SortedArrayTrie::build(vec![
            utf16("けん"),
            utf16("けんさく"),
            utf16("けんさ"),
            utf16("けんさつ"),
            utf16("けい"),
        ]);

        let words = decode_hits(&dict, dict.predictive_search(&utf16("けんさ")));

        assert_eq!(words, vec!["けんさ", "けんさく", "けんさつ"]);
    }

    #[test]
    fn predictive_search_returns_only_exact_prefix_branch() {
        let dict = SortedArrayTrie::build(vec![
            utf16("けん"),
            utf16("けんさく"),
            utf16("けんさ"),
            utf16("けんさつ"),
            utf16("けい"),
        ]);

        let words = decode_hits(&dict, dict.predictive_search(&utf16("けんさく")));

        assert_eq!(words, vec!["けんさく"]);
    }

    #[test]
    fn predictive_search_returns_empty_when_no_match_exists() {
        let dict = SortedArrayTrie::build(vec![utf16("けん"), utf16("けんさ"), utf16("けんさく")]);

        let words = decode_hits(&dict, dict.predictive_search(&utf16("けんし")));

        assert!(words.is_empty());
    }
}
