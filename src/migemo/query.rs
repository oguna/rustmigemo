use super::character_converter::*;
use super::compact_dictionary::*;
use super::regex_generator::*;
use super::romaji_processor::*;

pub fn query_a_word(word: &str, dict: &CompactDictionary, operator: &RegexOperator) -> String {
    let utf16word: Vec<u16> = word.encode_utf16().collect();
    let mut generator = RegexGenerator { root: None };
    generator.add(&utf16word);
    let lower: Vec<u16> = word.to_lowercase().encode_utf16().collect();
    /*
    for elem in dict.predictive_search(&lower) {
        generator.add(&elem);
    }
    */
    dict.predictive_search2(&lower, &mut generator);
    let zen: Vec<u16> = han2zen(word.to_string()).encode_utf16().collect();
    generator.add(&zen);
    let han: Vec<u16> = zen2han(word.to_string()).encode_utf16().collect();
    generator.add(&han);

    let processor = RomanProcessor::new();
    let hiragana = processor.romaji_to_hiragana_predictively(&lower);
    for suffix in hiragana.suffixes {
        let mut hira = hiragana.prefix.clone();
        hira.extend(suffix);
        generator.add(&hira);
        /*
        for elem in dict.predictive_search(&hira).iter() {
            generator.add(elem);
        }
        */
        dict.predictive_search2(&hira, &mut generator);
        let kata = hira2kata(&String::from_utf16_lossy(&hira));
        let u16kata: Vec<u16> = kata.encode_utf16().collect();
        generator.add(&u16kata);
        let u16zen = zen2han(kata).encode_utf16().collect();
        generator.add(&u16zen);
    }
    let generated = generator.generate(&operator);
    return generated;
}

pub fn query(word: String, dict: &CompactDictionary, operator: &RegexOperator) -> String {
    if word.len() == 0 {
        return "".to_string();
    }
    let words = parse_query(&word);
    let mut result = String::new();
    for w in words {
        result.extend(query_a_word(&w, dict, operator).chars());
    }
    return result;
}

pub struct QueryIter<'a> {
    string: &'a str,
    cursor: usize,
}

impl<'a> Iterator for QueryIter<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        let bytes = self.string.as_bytes();
        // カーソルが入力文字列の終端なら終了
        if bytes.len() <= self.cursor {
            return None;
        }
        // 空白をスキップする
        while self.cursor < bytes.len() && bytes[self.cursor] == 0x20 {
            self.cursor = self.cursor + 1;
        }
        // 単語の先頭文字の種類で場合分け
        let start = self.cursor;
        let c = bytes[self.cursor];
        if 0x41 <= c && c <= 0x5a {
            // 大文字なら、大文字または小文字が続くまで
            let mut next_char = bytes[self.cursor + 1];
            if 0x41 <= next_char && next_char <= 0x5a {
                while 0x41 <= next_char && next_char <= 0x5a {
                    self.cursor = self.cursor + 1;
                    if self.cursor + 1 < bytes.len() {
                        next_char = bytes[self.cursor + 1];
                    } else {
                        break;
                    }
                }
            } else if 0x61 <= next_char && next_char <= 0x7a {
                while 0x61 <= next_char && next_char <= 0x7a {
                    self.cursor = self.cursor + 1;
                    if self.cursor + 1 < bytes.len() {
                        next_char = bytes[self.cursor + 1];
                    } else {
                        break;
                    }
                }
            }
            self.cursor = self.cursor + 1;
            return Some(&self.string[start..self.cursor]);
        } else if 0x61 <= c && c <= 0x7a {
            // 小文字なら、小文字が続くまで
            let mut next_char = bytes[self.cursor + 1];
            while 0x61 <= next_char && next_char <= 0x7a {
                self.cursor = self.cursor + 1;
                if self.cursor + 1 < bytes.len() {
                    next_char = bytes[self.cursor + 1];
                } else {
                    break;
                }
            }
            self.cursor = self.cursor + 1;
            return Some(&self.string[start..self.cursor]);
        } else {
            // それ以外なら、空白に至るまで
            let mut next_char = bytes[self.cursor + 1];
            while next_char != 0x20 {
                self.cursor = self.cursor + 1;
                if self.cursor + 1 < bytes.len() {
                    next_char = bytes[self.cursor + 1];
                } else {
                    break;
                }
            }
            self.cursor = self.cursor + 1;
            return Some(&self.string[start..self.cursor]);
        }
    }
}

fn parse_query<'a>(query: &'a str) -> QueryIter<'a> {
    return QueryIter {
        string: query,
        cursor: 0,
    };
    /*
    // Regex is too heavy library
    let mut vec: Vec<String> = Vec::new();
    let re = Regex::new("[^A-Z\\s]+|[A-Z]{2,}|([A-Z][^A-Z\\s]+)|([A-Z]\\s*$)").unwrap();
    for cap in re.captures_iter(query) {
        vec.push(cap[0].to_string());
    }
    return vec;
    */
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query() {
        let query = "toukyouOosaka nagoyaFUKUOKAhokkaido";
        let mut iter = parse_query(query);
        assert_eq!(iter.next(), Some("toukyou"));
        assert_eq!(iter.next(), Some("Oosaka"));
        assert_eq!(iter.next(), Some("nagoya"));
        assert_eq!(iter.next(), Some("FUKUOKA"));
        assert_eq!(iter.next(), Some("hokkaido"));
        assert_eq!(iter.next(), None);
    }
}
