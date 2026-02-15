use super::character_converter::*;
use super::compact_dictionary::*;
use super::regex_generator::*;
use super::ternary_regex_generator::*;
use super::romaji_processor::RomajiProcessor;
use std::iter::Peekable;
use std::str::CharIndices;
use std::char::{decode_utf16, REPLACEMENT_CHARACTER};

pub fn query_a_word(word: &str, dict: &CompactDictionary, operator: &RegexOperator) -> String {
    query_a_word_with_generator(word, dict, operator, &mut TernaryRegexGenerator::new())
}

pub fn query_a_word_with_generator<T: RegexGeneratorTrait>(
    word: &str, 
    dict: &CompactDictionary, 
    operator: &RegexOperator,
    generator: &mut T
) -> String {
    let word_chars: Vec<char> = word.chars().collect();
    generator.add(&word_chars);
    
    let lower: Vec<u16> = word.to_lowercase().encode_utf16().collect();
    for elem in dict.predictive_search(&lower) {
        let elem_chars: Vec<char> = decode_utf16(elem.iter().cloned())
            .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
            .collect();
        generator.add(&elem_chars);
    }
    
    let zen_str = han2zen(word.to_string());
    let zen_chars: Vec<char> = zen_str.chars().collect();
    generator.add(&zen_chars);
    
    let han_str = zen2han(word.to_string());
    let han_chars: Vec<char> = han_str.chars().collect();
    generator.add(&han_chars);

    let processor = RomajiProcessor::new();
    let hiragana = processor.romaji_to_hiragana_predictively(&lower);
    for suffix in hiragana.suffixes {
        let mut hira = hiragana.prefix.clone();
        hira.extend(suffix);
        let hira_chars: Vec<char> = decode_utf16(hira.iter().cloned())
            .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
            .collect();
        generator.add(&hira_chars);
        
        for elem in dict.predictive_search(&hira) {
            let elem_chars: Vec<char> = decode_utf16(elem.iter().cloned())
                .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
                .collect();
            generator.add(&elem_chars);
        }
        
        let kata = hira2kata(&String::from_utf16_lossy(&hira));
        let kata_chars: Vec<char> = kata.chars().collect();
        generator.add(&kata_chars);
        
        let zen_kata = zen2han(kata);
        let zen_kata_chars: Vec<char> = zen_kata.chars().collect();
        generator.add(&zen_kata_chars);
    }
    let generated = generator.generate(&operator);
    return generated;
}

pub fn query(word: String, dict: &CompactDictionary, operator: &RegexOperator) -> String {
    if word.is_empty() {
        return "".to_string();
    }
    let mut result = String::new();
    for w in tokenize(&word) {
        result.push_str(&query_a_word(w, dict, operator));
    }
    result
}

/// クエリ文字列をトークンに分割するイテレータ
pub struct TokenizeIter<'a> {
    // 元の文字列全体への参照
    full_str: &'a str,
    // 文字とバイトインデックスを扱うためのイテレータ
    indices: Peekable<CharIndices<'a>>,
}

impl<'a> TokenizeIter<'a> {
    /// 新しいイテレータを作成します。
    fn new(input: &'a str) -> Self {
        TokenizeIter {
            full_str: input,
            indices: input.char_indices().peekable(),
        }
    }
}

impl<'a> Iterator for TokenizeIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // 1. トークンの開始位置を探す（先行する空白をスキップ）
        let start_index = match self.indices.find(|&(_, c)| !c.is_whitespace()) {
            Some((i, _)) => i,
            None => return None, // 文字列の終端に達したら終了
        };

        // 2. トークンの終了位置を探す
        // peek()で次の文字を「覗き見」しながら、トークンの区切りを判断する
        while let Some(&(current_index, current_char)) = self.indices.peek() {
            // 区切り文字（空白）が見つかったら、そこでトークンは終了
            if current_char.is_whitespace() {
                break;
            }

            // 前の文字の種類を取得
            // start_indexからcurrent_indexまでの部分文字列の最後の文字
            let prev_char = self.full_str[..current_index].chars().last().unwrap_or(' ');

            // トークンの区切りルールの判定
            // [修正] is_ascii_alphabetic から is_ascii_alphanumeric に変更し、数字もトークンに含めるようにした
            let is_boundary =
                // ルール1: 小文字の後に大文字が来た場合 (例: "toukyouO"saka)
                (prev_char.is_lowercase() && current_char.is_uppercase()) ||
                // ルール2: 2文字以上の大文字の後に小文字が来た場合 (例: "FUKUOKAh"okkaido)
                (prev_char.is_uppercase() && current_char.is_lowercase() && {
                    // "FUKUOKA" のように大文字が続いているか確認
                    let token_so_far = &self.full_str[start_index..current_index];
                    token_so_far.chars().count() > 1 && token_so_far.chars().all(|c| c.is_uppercase())
                }) ||
                // ルール3: 非ASCII英数字の後にASCII英数字が来た場合 (例: "東京T"ower, "東京123")
                (!prev_char.is_ascii_alphanumeric() && current_char.is_ascii_alphanumeric()) ||
                // ルール4: ASCII英数字の後に非ASCII英数字が来た場合 (例: "Tower東京", "word1東京")
                (prev_char.is_ascii_alphanumeric() && !current_char.is_ascii_alphanumeric());

            if is_boundary {
                break; // 区切りなのでループを抜ける
            }

            // 区切りでなければ、イテレータを1つ進める
            self.indices.next();
        }

        // 3. トークンを切り出して返す
        let end_index = self.indices.peek().map_or(self.full_str.len(), |&(i, _)| i);
        Some(&self.full_str[start_index..end_index])
    }
}

pub fn tokenize(input: &str) -> TokenizeIter<'_> {
    TokenizeIter::new(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let query = "toukyouOosaka nagoyaFUKUOKAhokkaido ";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(
            tokens,
            vec!["toukyou", "Oosaka", "nagoya", "FUKUOKA", "hokkaido"]
        );
    }
    #[test]
    fn test_tokenize_1() {
        let query = "a";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(tokens, vec!["a"]);
    }

    #[test]
    fn test_tokenize_2() {
        let query = "A";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(tokens, vec!["A"]);
    }

    #[test]
    fn test_tokenize_3() {
        let query = "あ";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(tokens, vec!["あ"]);
    }

    #[test]
    fn test_tokenize_4() {
        let query = "/";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(tokens, vec!["/"]);
    }

    #[test]
    fn test_tokenize_5() {
        let query = "aaA";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(tokens, vec!["aa", "A"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let query = "";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_tokenize_whitespace() {
        let query = "  word1   word2 ";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(tokens, vec!["word1", "word2"]);
    }

    #[test]
    fn test_tokenize_mixed() {
        let query = "東京Tower";
        let tokens: Vec<&str> = tokenize(query).collect();
        assert_eq!(tokens, vec!["東京", "Tower"]);
    }

    #[test]
    fn test_generator_compatibility() {
        // 両方のジェネレータが単純な文字列で同じ結果を生成することを確認
        let mut gen1 = RegexGenerator { root: None };
        let mut gen2 = TernaryRegexGenerator::new();
        let op = RegexOperator::Default;
        
        let test_chars: Vec<char> = "test".chars().collect();
        gen1.add(&test_chars);
        gen2.add(&test_chars);
        
        let result1 = gen1.generate(&op);
        let result2 = gen2.generate(&op);
        
        assert_eq!(result1, result2);
        assert_eq!(result1, "test");
    }
}
