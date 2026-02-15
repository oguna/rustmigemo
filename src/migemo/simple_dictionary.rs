pub struct SimpleDictionary {
    keys: Vec<Vec<u16>>,
    values: Vec<Vec<u16>>,
}

impl SimpleDictionary {
    pub fn build(file: String) -> SimpleDictionary {
        let lines = file.lines();
        let mut pairs = Vec::<(String, String)>::new();
        for line in lines {
            if !line.starts_with(";") && line.len() != 0 {
                let semicolon_pos_opt = line.find('\t');
                if semicolon_pos_opt.is_some() {
                    let semicolon_pos = semicolon_pos_opt.unwrap();
                    let key = line[0..semicolon_pos].to_string();
                    let value = line[semicolon_pos+1..].to_string();
                    pairs.push((key, value))
                }
            }
        }
        pairs.sort_unstable_by_key(|k| k.0.to_string());
        let mut keys = Vec::<Vec<u16>>::with_capacity(pairs.len());
        let mut values = Vec::<Vec<u16>>::with_capacity(pairs.len());
        for (k,v) in pairs {
            values.push(v.encode_utf16().collect());
            keys.push(k.encode_utf16().collect());
        }
        return SimpleDictionary {
            keys,
            values,
        };
    }

    pub fn predictive_search(&self, hiragana: &Vec<u16>) -> Vec<Vec<u16>> {
        if hiragana.len() > 0 {
            let mut stop = hiragana.clone();
            let stop_char = stop[&stop.len()-1]+1;
            let pos = stop.len()-1;
            stop[pos] = stop_char;
            let start_pos_result = self.keys.binary_search(hiragana);
            let start_pos = match start_pos_result {
                Ok(i) => i,
                Err(i) => i,
            };
            let end_pos_result = self.keys.binary_search(&stop);
            let end_pos = match end_pos_result {
                Ok(i) => i,
                Err(i) => i,
            };
            let mut result = Vec::<Vec<u16>>::new();
            for i in start_pos..end_pos {
                for word in String::from_utf16(&self.values[i]).unwrap().split("\t") {
                    result.push(word.encode_utf16().collect());
                }
            }
            return result;
        } else {
            return vec![];
        }
    }
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_1() {
        let text = "けんさ\t検査\nけんさく\t検索\t研削";
        let dict = SimpleDictionary::build(text.to_string());
        let words = dict.predictive_search(&"けんさ".encode_utf16().collect());
        assert_eq!(String::from_utf16(&words.concat()).unwrap(), "検査検索研削");
	}
    
	#[test]
	fn test_2() {
        let text = "けんさ\t検査\nけんさく\t検索\t研削";
        let dict = SimpleDictionary::build(text.to_string());
        let words = dict.predictive_search(&"けんさく".encode_utf16().collect());
        assert_eq!(String::from_utf16(&words.concat()).unwrap(), "検索研削");
	}
}
