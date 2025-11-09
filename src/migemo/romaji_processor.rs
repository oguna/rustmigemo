use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Debug, Clone)]
struct RomanEntry<'a> {
    roman: &'a [u16],
    hiragana: &'a [u16],
    remain: usize,
    index: u32,
}

impl<'a> RomanEntry<'a> {
    fn _calculate_index(roman: &[u16], start: usize, end: usize) -> u32 {
        let mut result: u32 = 0;
        const MAX_CHARS_FOR_INDEX: usize = 4;
        const BITS_PER_CHAR: u32 = 8;
        for i in 0..MAX_CHARS_FOR_INDEX {
            let index = i + start;
            let c: u16 = if index < roman.len() && index < end {
                roman[index]
            } else {
                0
            };
            result |= c as u32;
            if i < MAX_CHARS_FOR_INDEX - 1 {
                result <<= BITS_PER_CHAR;
            }
        }
        result
    }

    pub fn calculate_index(roman: &[u16]) -> u32 {
        RomanEntry::_calculate_index(roman, 0, 4)
    }
}

static ROMAN_TABLE: OnceLock<(Vec<RomanEntry<'static>>, Vec<u32>)> = OnceLock::new();

fn get_roman_table() -> &'static (Vec<RomanEntry<'static>>, Vec<u32>) {
    ROMAN_TABLE.get_or_init(|| {
        let mut entries: Vec<RomanEntry<'static>> = ROMAN_ENTRIES.iter()
            .map(|(roman_str, hiragana_str, remain)| {
                let roman: &'static [u16] = Box::leak(roman_str.encode_utf16().collect::<Vec<_>>().into_boxed_slice());
                let hiragana: &'static [u16] = Box::leak(hiragana_str.encode_utf16().collect::<Vec<_>>().into_boxed_slice());
                
                RomanEntry {
                    roman,
                    hiragana,
                    remain: *remain,
                    index: RomanEntry::calculate_index(roman),
                }
            })
            .collect();

        entries.sort_by_key(|e| e.index);

        let indices = entries.iter().map(|e| e.index).collect();

        (entries, indices)
    })
}


pub struct RomajiPredictiveResult {
    pub prefix: Vec<u16>,
    pub suffixes: Vec<Vec<u16>>,
}

pub struct RomanProcessor {
    entries: &'static [RomanEntry<'static>],
    indices: &'static [u32],
}

impl RomanProcessor {
    pub fn new() -> RomanProcessor {
        let table = get_roman_table();
        RomanProcessor {
            entries: &table.0,
            indices: &table.1,
        }
    }

    fn binary_search(a: &[u32], from_index: usize, to_index: usize, key: u32) -> isize {
        let mut low = from_index as isize;
        let mut high = to_index as isize - 1;
        while low <= high {
            let mid = low + (high - low) / 2;
            let mid_val = a[mid as usize];
            if mid_val < key {
                low = mid + 1;
            } else if mid_val > key {
                high = mid - 1;
            } else {
                return mid;
            }
        }
        -(low + 1)
    }

    pub fn romaji_to_hiragana(&self, romaji: &[u16]) -> Vec<u16> {
        if romaji.is_empty() {
            return Vec::new();
        }
        let mut hiragana = Vec::new();
        let mut start = 0;
        let mut end = 1;
        while start < romaji.len() {
            let mut last_found: isize = -1;
            let mut lower: isize = 0;
            let mut upper: isize = self.indices.len() as isize;
            while upper - lower > 1 && end <= romaji.len() {
                let lower_key = RomanEntry::_calculate_index(romaji, start, end);
                lower = RomanProcessor::binary_search(self.indices, lower as usize, upper as usize, lower_key);
                if lower >= 0 {
                    last_found = lower;
                } else {
                    lower = -lower - 1;
                }
                let upper_key = lower_key + (1 << (32 - 8 * (end - start)));
                upper = RomanProcessor::binary_search(self.indices, lower as usize, upper as usize, upper_key);
                if upper < 0 {
                    upper = -upper - 1;
                }
                end += 1;
            }
            if last_found >= 0 {
                let entry = &self.entries[last_found as usize];
                hiragana.extend_from_slice(entry.hiragana);
                start += entry.roman.len() - entry.remain;
                end = start + 1;
            } else {
                hiragana.push(romaji[start]);
                start += 1;
                end = start + 1;
            }
        }
        hiragana
    }

    fn find_roman_entry_predicatively(indices: &[u32], roman: &[u16], offset: usize) -> Vec<usize>  {
        let mut start_index: isize = 0;
        let mut end_index: isize = indices.len() as isize;
        for i in 0..4 {
            if roman.len() <= offset + i {
                break;
            }
            let start_key = RomanEntry::_calculate_index(roman, offset, offset + i + 1);
            start_index = RomanProcessor::binary_search(indices, start_index as usize, end_index as usize, start_key);
            if start_index < 0 {
                start_index = -start_index - 1;
            }
            let end_key = start_key + (1 << (24 - 8 * i));
            end_index = RomanProcessor::binary_search(indices, start_index as usize, end_index as usize, end_key);
            if end_index < 0 {
                end_index = -end_index - 1;
            }
            if end_index - start_index == 1 {
                return vec![start_index as usize];
            }
        }
        (start_index..end_index).map(|i| i as usize).collect()
    }

    pub fn romaji_to_hiragana_predictively(&self, romaji: &[u16]) -> RomajiPredictiveResult {
        if romaji.is_empty() {
            return RomajiPredictiveResult {
                prefix: vec![],
                suffixes: vec![vec![]],
            };
        }
        let mut hiragana = Vec::new();
        let mut start = 0;
        let mut end = 1;
        while start < romaji.len() {
            let mut last_found: isize = -1;
            let mut lower: isize = 0;
            let mut upper: isize = self.indices.len() as isize;
            while upper - lower > 1 && end <= romaji.len() {
                let lower_key = RomanEntry::_calculate_index(romaji, start, end);
                lower = RomanProcessor::binary_search(self.indices, lower as usize, upper as usize, lower_key);
                if lower >= 0 {
                    last_found = lower;
                } else {
                    lower = -lower - 1;
                }
                let upper_key = lower_key + (1 << (32 - 8 * (end - start)));
                upper = RomanProcessor::binary_search(self.indices, lower as usize, upper as usize, upper_key);
                if upper < 0 {
                    upper = -upper - 1;
                }
                end += 1;
            }
            if end > romaji.len() && upper - lower > 1 {
                let mut set: HashSet<Vec<u16>> = HashSet::new();
                for i in lower..upper {
                    let re = &self.entries[i as usize];
                    if re.remain > 0 {
                        // Check for underflow before subtraction
                        if end > re.remain {
                            let set2 = RomanProcessor::find_roman_entry_predicatively(self.indices ,romaji, end - 1 - re.remain);
                            for re2_idx in set2 {
                                let re2 = &self.entries[re2_idx];
                                if re2.remain == 0 {
                                    let mut combined = re.hiragana.to_vec();
                                    combined.extend_from_slice(re2.hiragana);
                                    set.insert(combined);
                                }
                            }
                        }
                    } else {
                        set.insert(re.hiragana.to_vec());
                    }
                }
                return RomajiPredictiveResult {
                    prefix: hiragana,
                    suffixes: set.into_iter().collect(),
                };
            }
            if last_found >= 0 {
                let entry = &self.entries[last_found as usize];
                hiragana.extend_from_slice(entry.hiragana);
                start += entry.roman.len() - entry.remain;
                end = start + 1;
            } else {
                hiragana.push(romaji[start]);
                start += 1;
                end = start + 1;
            }
        }
        RomajiPredictiveResult {
            prefix: hiragana,
            suffixes: vec![vec![]],
        }
    }
}

const ROMAN_ENTRIES: [(&str, &str, usize); 312] = [
    ("-", "ー", 0),
    ("~", "〜", 0),
    (".", "。", 0),
    (",", "、", 0),
    ("z/", "・", 0),
    ("z.", "…", 0),
    ("z,", "‥", 0),
    ("zh", "←", 0),
    ("zj", "↓", 0),
    ("zk", "↑", 0),
    ("zl", "→", 0),
    ("z-", "〜", 0),
    ("z[", "『", 0),
    ("z]", "』", 0),
    ("[", "「", 0),
    ("]", "」", 0),
    ("va", "ゔぁ", 0),
    ("vi", "ゔぃ", 0),
    ("vu", "ゔ", 0),
    ("ve", "ゔぇ", 0),
    ("vo", "ゔぉ", 0),
    ("vya", "ゔゃ", 0),
    ("vyi", "ゔぃ", 0),
    ("vyu", "ゔゅ", 0),
    ("vye", "ゔぇ", 0),
    ("vyo", "ゔょ", 0),
    ("qq", "っ", 1),
    ("vv", "っ", 1),
    ("ll", "っ", 1),
    ("xx", "っ", 1),
    ("kk", "っ", 1),
    ("gg", "っ", 1),
    ("ss", "っ", 1),
    ("zz", "っ", 1),
    ("jj", "っ", 1),
    ("tt", "っ", 1),
    ("dd", "っ", 1),
    ("hh", "っ", 1),
    ("ff", "っ", 1),
    ("bb", "っ", 1),
    ("pp", "っ", 1),
    ("mm", "っ", 1),
    ("yy", "っ", 1),
    ("rr", "っ", 1),
    ("ww", "っ", 1),
    ("www", "w", 2),
    ("cc", "っ", 1),
    ("kya", "きゃ", 0),
    ("kyi", "きぃ", 0),
    ("kyu", "きゅ", 0),
    ("kye", "きぇ", 0),
    ("kyo", "きょ", 0),
    ("gya", "ぎゃ", 0),
    ("gyi", "ぎぃ", 0),
    ("gyu", "ぎゅ", 0),
    ("gye", "ぎぇ", 0),
    ("gyo", "ぎょ", 0),
    ("sya", "しゃ", 0),
    ("syi", "しぃ", 0),
    ("syu", "しゅ", 0),
    ("sye", "しぇ", 0),
    ("syo", "しょ", 0),
    ("sha", "しゃ", 0),
    ("shi", "し", 0),
    ("shu", "しゅ", 0),
    ("she", "しぇ", 0),
    ("sho", "しょ", 0),
    ("zya", "じゃ", 0),
    ("zyi", "じぃ", 0),
    ("zyu", "じゅ", 0),
    ("zye", "じぇ", 0),
    ("zyo", "じょ", 0),
    ("tya", "ちゃ", 0),
    ("tyi", "ちぃ", 0),
    ("tyu", "ちゅ", 0),
    ("tye", "ちぇ", 0),
    ("tyo", "ちょ", 0),
    ("cha", "ちゃ", 0),
    ("chi", "ち", 0),
    ("chu", "ちゅ", 0),
    ("che", "ちぇ", 0),
    ("cho", "ちょ", 0),
    ("cya", "ちゃ", 0),
    ("cyi", "ちぃ", 0),
    ("cyu", "ちゅ", 0),
    ("cye", "ちぇ", 0),
    ("cyo", "ちょ", 0),
    ("dya", "ぢゃ", 0),
    ("dyi", "ぢぃ", 0),
    ("dyu", "ぢゅ", 0),
    ("dye", "ぢぇ", 0),
    ("dyo", "ぢょ", 0),
    ("tsa", "つぁ", 0),
    ("tsi", "つぃ", 0),
    ("tse", "つぇ", 0),
    ("tso", "つぉ", 0),
    ("tha", "てゃ", 0),
    ("thi", "てぃ", 0),
    ("t'i", "てぃ", 0),
    ("thu", "てゅ", 0),
    ("the", "てぇ", 0),
    ("tho", "てょ", 0),
    ("t'yu", "てゅ", 0),
    ("dha", "でゃ", 0),
    ("dhi", "でぃ", 0),
    ("d'i", "でぃ", 0),
    ("dhu", "でゅ", 0),
    ("dhe", "でぇ", 0),
    ("dho", "でょ", 0),
    ("d'yu", "でゅ", 0),
    ("twa", "とぁ", 0),
    ("twi", "とぃ", 0),
    ("twu", "とぅ", 0),
    ("twe", "とぇ", 0),
    ("two", "とぉ", 0),
    ("t'u", "とぅ", 0),
    ("dwa", "どぁ", 0),
    ("dwi", "どぃ", 0),
    ("dwu", "どぅ", 0),
    ("dwe", "どぇ", 0),
    ("dwo", "どぉ", 0),
    ("d'u", "どぅ", 0),
    ("nya", "にゃ", 0),
    ("nyi", "にぃ", 0),
    ("nyu", "にゅ", 0),
    ("nye", "にぇ", 0),
    ("nyo", "にょ", 0),
    ("hya", "ひゃ", 0),
    ("hyi", "ひぃ", 0),
    ("hyu", "ひゅ", 0),
    ("hye", "ひぇ", 0),
    ("hyo", "ひょ", 0),
    ("bya", "びゃ", 0),
    ("byi", "びぃ", 0),
    ("byu", "びゅ", 0),
    ("bye", "びぇ", 0),
    ("byo", "びょ", 0),
    ("pya", "ぴゃ", 0),
    ("pyi", "ぴぃ", 0),
    ("pyu", "ぴゅ", 0),
    ("pye", "ぴぇ", 0),
    ("pyo", "ぴょ", 0),
    ("fa", "ふぁ", 0),
    ("fi", "ふぃ", 0),
    ("fu", "ふ", 0),
    ("fe", "ふぇ", 0),
    ("fo", "ふぉ", 0),
    ("fya", "ふゃ", 0),
    ("fyu", "ふゅ", 0),
    ("fyo", "ふょ", 0),
    ("hwa", "ふぁ", 0),
    ("hwi", "ふぃ", 0),
    ("hwe", "ふぇ", 0),
    ("hwo", "ふぉ", 0),
    ("hwyu", "ふゅ", 0),
    ("mya", "みゃ", 0),
    ("myi", "みぃ", 0),
    ("myu", "みゅ", 0),
    ("mye", "みぇ", 0),
    ("myo", "みょ", 0),
    ("rya", "りゃ", 0),
    ("ryi", "りぃ", 0),
    ("ryu", "りゅ", 0),
    ("rye", "りぇ", 0),
    ("ryo", "りょ", 0),
    ("n'", "ん", 0),
    ("nn", "ん", 0),
    ("n", "ん", 0),
    ("xn", "ん", 0),
    ("a", "あ", 0),
    ("i", "い", 0),
    ("u", "う", 0),
    ("wu", "う", 0),
    ("e", "え", 0),
    ("o", "お", 0),
    ("xa", "ぁ", 0),
    ("xi", "ぃ", 0),
    ("xu", "ぅ", 0),
    ("xe", "ぇ", 0),
    ("xo", "ぉ", 0),
    ("la", "ぁ", 0),
    ("li", "ぃ", 0),
    ("lu", "ぅ", 0),
    ("le", "ぇ", 0),
    ("lo", "ぉ", 0),
    ("lyi", "ぃ", 0),
    ("xyi", "ぃ", 0),
    ("lye", "ぇ", 0),
    ("xye", "ぇ", 0),
    ("ye", "いぇ", 0),
    ("ka", "か", 0),
    ("ki", "き", 0),
    ("ku", "く", 0),
    ("ke", "け", 0),
    ("ko", "こ", 0),
    ("xka", "ヵ", 0),
    ("xke", "ヶ", 0),
    ("lka", "ヵ", 0),
    ("lke", "ヶ", 0),
    ("ga", "が", 0),
    ("gi", "ぎ", 0),
    ("gu", "ぐ", 0),
    ("ge", "げ", 0),
    ("go", "ご", 0),
    ("sa", "さ", 0),
    ("si", "し", 0),
    ("su", "す", 0),
    ("se", "せ", 0),
    ("so", "そ", 0),
    ("ca", "か", 0),
    ("ci", "し", 0),
    ("cu", "く", 0),
    ("ce", "せ", 0),
    ("co", "こ", 0),
    ("qa", "くぁ", 0),
    ("qi", "くぃ", 0),
    ("qu", "く", 0),
    ("qe", "くぇ", 0),
    ("qo", "くぉ", 0),
    ("kwa", "くぁ", 0),
    ("kwi", "くぃ", 0),
    ("kwu", "くぅ", 0),
    ("kwe", "くぇ", 0),
    ("kwo", "くぉ", 0),
    ("gwa", "ぐぁ", 0),
    ("gwi", "ぐぃ", 0),
    ("gwu", "ぐぅ", 0),
    ("gwe", "ぐぇ", 0),
    ("gwo", "ぐぉ", 0),
    ("za", "ざ", 0),
    ("zi", "じ", 0),
    ("zu", "ず", 0),
    ("ze", "ぜ", 0),
    ("zo", "ぞ", 0),
    ("ja", "じゃ", 0),
    ("ji", "じ", 0),
    ("ju", "じゅ", 0),
    ("je", "じぇ", 0),
    ("jo", "じょ", 0),
    ("jya", "じゃ", 0),
    ("jyi", "じぃ", 0),
    ("jyu", "じゅ", 0),
    ("jye", "じぇ", 0),
    ("jyo", "じょ", 0),
    ("ta", "た", 0),
    ("ti", "ち", 0),
    ("tu", "つ", 0),
    ("tsu", "つ", 0),
    ("te", "て", 0),
    ("to", "と", 0),
    ("da", "だ", 0),
    ("di", "ぢ", 0),
    ("du", "づ", 0),
    ("de", "で", 0),
    ("do", "ど", 0),
    ("xtu", "っ", 0),
    ("xtsu", "っ", 0),
    ("ltu", "っ", 0),
    ("ltsu", "っ", 0),
    ("na", "な", 0),
    ("ni", "に", 0),
    ("nu", "ぬ", 0),
    ("ne", "ね", 0),
    ("no", "の", 0),
    ("ha", "は", 0),
    ("hi", "ひ", 0),
    ("hu", "ふ", 0),
    ("fu", "ふ", 0),
    ("he", "へ", 0),
    ("ho", "ほ", 0),
    ("ba", "ば", 0),
    ("bi", "び", 0),
    ("bu", "ぶ", 0),
    ("be", "べ", 0),
    ("bo", "ぼ", 0),
    ("pa", "ぱ", 0),
    ("pi", "ぴ", 0),
    ("pu", "ぷ", 0),
    ("pe", "ぺ", 0),
    ("po", "ぽ", 0),
    ("ma", "ま", 0),
    ("mi", "み", 0),
    ("mu", "む", 0),
    ("me", "め", 0),
    ("mo", "も", 0),
    ("xya", "ゃ", 0),
    ("lya", "ゃ", 0),
    ("ya", "や", 0),
    ("wyi", "ゐ", 0),
    ("xyu", "ゅ", 0),
    ("lyu", "ゅ", 0),
    ("yu", "ゆ", 0),
    ("wye", "ゑ", 0),
    ("xyo", "ょ", 0),
    ("lyo", "ょ", 0),
    ("yo", "よ", 0),
    ("ra", "ら", 0),
    ("ri", "り", 0),
    ("ru", "る", 0),
    ("re", "れ", 0),
    ("ro", "ろ", 0),
    ("xwa", "ゎ", 0),
    ("lwa", "ゎ", 0),
    ("wa", "わ", 0),
    ("wi", "うぃ", 0),
    ("we", "うぇ", 0),
    ("wo", "を", 0),
    ("wha", "うぁ", 0),
    ("whi", "うぃ", 0),
    ("whu", "う", 0),
    ("whe", "うぇ", 0),
    ("who", "うぉ", 0),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn romaji_to_hiragana(romaji: &str) -> String {
        let processor = RomanProcessor::new();
        let kensaku: Vec<u16> = romaji.encode_utf16().collect();
        let actual: Vec<u16> = processor.romaji_to_hiragana(&kensaku);
        String::from_utf16(&actual).unwrap()
    }

    #[test]
    fn romaji_to_hiragana_1() {
        let actual = romaji_to_hiragana("kensaku");
        let expected = "けんさく";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_2() {
        let actual = romaji_to_hiragana("atti");
        let expected = "あっち";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_3() {
        let actual = romaji_to_hiragana("att");
        let expected = "あっt";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_4() {
        let actual = romaji_to_hiragana("www");
        let expected = "wっw";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_5() {
        let actual = romaji_to_hiragana("kk");
        let expected = "っk";
        assert_eq!(actual, expected);
    }

    #[test]
    fn romaji_to_hiragana_6() {
        let actual = romaji_to_hiragana("n");
        let expected = "ん";
        assert_eq!(actual, expected);
    }

        fn romaji_to_hiragana_predictively(romaji: &str) -> (String, Vec<String>) {
        let processor = RomanProcessor::new();
        let kensaku: Vec<u16> = romaji.encode_utf16().collect();
        let actual = processor.romaji_to_hiragana_predictively(&kensaku);
        let prefix = String::from_utf16(&actual.prefix).unwrap();
        let mut suffixes: Vec<String> = actual.suffixes.iter().map(|x| String::from_utf16(x).unwrap()).collect();
        suffixes.sort();
        (prefix, suffixes)
    }

    #[test]
    fn romaji_to_hiragana_predictively_1() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("kiku");
        assert_eq!(prefix, "きく");
        assert_eq!(suffixes.len(), 1);
        assert_eq!(suffixes[0], "");
    }

    #[test]
    fn romaji_to_hiragana_predictively_2() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("ky");
        let mut expected_suffixes = vec!["きゃ", "きぃ", "きぇ", "きゅ", "きょ"];
        expected_suffixes.sort();
        assert_eq!(prefix, "");
        assert_eq!(suffixes, expected_suffixes);
    }

    #[test]
    fn romaji_to_hiragana_predictively_3() {        let (prefix, suffixes) = romaji_to_hiragana_predictively("kky");
        let mut expected_suffixes = vec!["きゃ", "きぃ", "きぇ", "きゅ", "きょ"];
        expected_suffixes.sort();
        assert_eq!(prefix, "っ");
        assert_eq!(suffixes, expected_suffixes);
    }

    #[test]
    fn romaji_to_hiragana_predictively_4() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("n");
        let mut expected_suffixes = vec!["にょ", "の", "にゃ", "ぬ", "ね", "な", "にぇ", "にゅ", "に", "ん", "にぃ"];
        expected_suffixes.sort();
        assert_eq!(prefix, "");
        assert_eq!(suffixes, expected_suffixes);
    }

    #[test]
    fn romaji_to_hiragana_predictively_5() {
        let (prefix, suffixes) = romaji_to_hiragana_predictively("denk");
        assert_eq!(prefix, "でん");
        assert!(suffixes.iter().any(|s| s == "か"));
    }

    #[test]
    fn romaji_to_hiragana_w() {
        let (_, _) = romaji_to_hiragana_predictively("w");
    }
}
