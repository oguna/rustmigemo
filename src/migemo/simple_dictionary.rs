pub struct SimpleDictionary {
    keys: Vec<String>,
    values: Vec<String>,
}

impl SimpleDictionary {
    pub fn build(file: String) -> &SimpleDictionary {
        let mut lines = file.lines();
        let mut pairs = Vec<(String, String)>();
        for line in lines {
            if !line.starts_with(";") && line.len() != 0 {
                let semicolonPos = line.find('\t');
                let key = line[0...semicolonPos];
                let value = line[semicolonPos+1...];
                pairs.push((key, value))
            }
        }
        pairs.sort_unstable_by_key(|k| k[0]);
        let keys = pairs.into_iter().map(|x| x[0]).collect();
        let values = pairs.into_iter().map(|x| x[1]).collect();
        return &SimpleDictionary {
            keys,
            values,
        };
    }

    pub fn predictive_search(hiragana: String) -> Vec<String> {
        if hiragana.len() > 0 {
            let stop = hiragana.substring(0, hiragana.length - 1) + String.fromCodePoint(hiragana.codePointAt(hiragana.length - 1)||0 + 1);
            let startPos = binarySearchString(this.keys, 0, this.keys.length, hiragana);
            if startPos < 0 {
                startPos = -(startPos + 1);
            }
            let endPos = binarySearchString(this.keys, 0, this.keys.length, stop);
            if endPos < 0 {
                endPos = -(endPos + 1);
            }
            let result = Array<string>();
            for (let i = startPos; i < endPos; i++) {
                for (let j of this.values[i].split("\t")) {
                    result.push(j);
                }
            }
            return result;
        } else {
            return [];
        }
    }
}