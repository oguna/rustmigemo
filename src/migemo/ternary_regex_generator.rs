use super::regex_generator::{RegexOperator, RegexOperatorDetail, RegexGeneratorTrait};

#[derive(Debug)]
pub struct TernaryRegexNode {
	pub code: char,
	pub child: Option<Box<TernaryRegexNode>>,
	pub left: Option<Box<TernaryRegexNode>>,
	pub right: Option<Box<TernaryRegexNode>>,
    pub level: usize,
}

#[derive(Debug)]
pub struct TernaryRegexGenerator {
    pub root: Option<Box<TernaryRegexNode>>,
    // TODO: pub unused: Option<Box<TernaryRegexNode>>,
}

fn skew(t: Option<Box<TernaryRegexNode>>) -> Option<Box<TernaryRegexNode>> {
    match t {
        None => {
            return None;
        },
        Some(mut t1) => {
            if let Some(left) = &t1.left {
                if left.level == t1.level {
                    let mut l = t1.left.take().unwrap();
                    t1.left = l.right.take();
                    l.right = Some(t1);
                    return Some(l);
                }
            }
            Some(t1)
        }
    }
}

fn split(t: Option<Box<TernaryRegexNode>>) -> Option<Box<TernaryRegexNode>> {
    let mut t = match t {
        Some(node) => node,
        None => return None,
    };

    if let Some(right) = t.right.as_ref() {
        if let Some(right_right) = right.right.as_ref() {
            if t.level == right_right.level {
                let mut r = t.right.take().unwrap();
                t.right = r.left.take();
                r.left = Some(t);
                r.level += 1;
                return Some(r);
            }
        }
    }

    Some(t)
}

fn insert(word: &[char], offset: usize, t: Option<Box<TernaryRegexNode>>) -> Option<Box<TernaryRegexNode>> {
    if offset >= word.len() {
        return t;
    }
    match t {
        None => {
            let mut r = Box::new(TernaryRegexNode {
				code: word[offset],
                level: 1,
                left: None,
                right: None,
                child: None
			});
            r.child = insert(word, offset + 1, None);
            return Some(r);
        },
        Some(mut tt) => {
            let x = word[offset];
            if x < tt.code {
                tt.left = insert(word, offset, tt.left);
            } else if x > tt.code {
                tt.right = insert(word, offset, tt.right)
            } else {
                if tt.child.is_some() {
                    tt.child = insert(word, offset+1, tt.child);
                }
                return Some(tt);
            };
            return split(skew(Some(tt)));
        }
    }
}

fn traverse_siblings<'a>(node: &'a Option<Box<TernaryRegexNode>>, buffer: &mut Vec<&'a Box<TernaryRegexNode>>) {
    match node {
        None => {
        },
        Some(ref n) => {
            traverse_siblings(&n.left, buffer);
            buffer.push(n);
            traverse_siblings(&n.right, buffer);
        }
    }
}

const ESCAPE_BITMAP: [u64; 2] = generate_escape_bitmap();

const fn generate_escape_bitmap() -> [u64; 2] {
    const ESCAPE_CHARS: &str = r"()[]{}?*+|^$.\";
    let mut bitmap = [0u64; 2];
    let bytes = ESCAPE_CHARS.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let byte = bytes[i];
        let c = byte as u64;
        if c < 128 {
            bitmap[(c / 64) as usize] |= 1 << (c % 64);
        }
        i += 1;
    }
    bitmap
}

fn is_characters_to_escape(c: char) -> bool {
    if c < '\u{80}' {
        return (ESCAPE_BITMAP[(c as usize)/64]>>((c as u64)%64))&1 == 1;
    } else {
        return false;
    }
}

fn generate(node: &Option<Box<TernaryRegexNode>>, buffer: &mut String, op: &RegexOperatorDetail) {
    let mut brother = 0;
    let mut haschild = 0;
    let mut siblings = Vec::<&Box<TernaryRegexNode>>::new();
    traverse_siblings(node, &mut siblings);
    for n in &siblings {
        brother += 1;
        if n.child.is_some() {
            haschild += 1;
        }
    }
    let nochild = brother - haschild;
    if brother > 1 && haschild > 0 {
        buffer.push_str(&op.begin_group);
    }
    if nochild > 0 {
        if nochild > 1 {
            buffer.push_str(&op.begin_class);
        }
        for n in &siblings {
            if n.child.is_some() {
                continue;
            }
            if is_characters_to_escape(n.code) {
                buffer.push('\\');
            }
            buffer.push(n.code);
        }
        if nochild > 1 {
            buffer.push_str(&op.end_class);
        }
    }
    if haschild > 0 {
        if nochild > 0 {
            buffer.push_str(&op.or);
        }
        let mut is_first = true;
        for n in &siblings {
            if n.child.is_some() {
                if !is_first {
                    buffer.push_str(&op.or);
                }
                if is_characters_to_escape(n.code) {
                    buffer.push('\\');
                }
                buffer.push(n.code);
                generate(&n.child, buffer, op);
                is_first = false;
            }
        }
    }
    if brother > 1 && haschild > 0 {
        buffer.push_str(&op.end_group);
    }
}

impl TernaryRegexGenerator {
    pub fn new() -> TernaryRegexGenerator {
        return TernaryRegexGenerator {
            root: None,
        }
    }

    pub fn add(&mut self, word: &[char]) {
        if word.len() == 0 {
            return;
        }
        self.root = insert(word, 0, self.root.take());
    }

    pub fn generate(&self, op: &RegexOperator) -> String {
        if self.root.is_none() {
            return String::new();
        } else {
            let op_detail = RegexOperatorDetail::get_regex_operator_detail(op);
            let mut buffer = String::new();
            generate(& self.root, &mut buffer, &op_detail);
            return buffer;
        }
    }
}

impl RegexGeneratorTrait for TernaryRegexGenerator {
    fn add(&mut self, word: &[char]) {
        if word.len() == 0 {
            return;
        }
        self.root = insert(word, 0, self.root.take());
    }

    fn generate(&self, op: &RegexOperator) -> String {
        if self.root.is_none() {
            return String::new();
        } else {
            let op_detail = RegexOperatorDetail::get_regex_operator_detail(op);
            let mut buffer = String::new();
            generate(& self.root, &mut buffer, &op_detail);
            return buffer;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_test(words: &[&str], expected: &str) {
        let mut rxgen = TernaryRegexGenerator::new();
        let rxop = RegexOperator::Default;

        for word in words {
            let word_chars: Vec<char> = word.chars().collect();
            rxgen.add(&word_chars);
        }

        let actual = rxgen.generate(&rxop);
        assert_eq!(actual, expected);
    }

    #[test]
    fn bad_dad() {
        run_test(&["bad", "dad"], "(bad|dad)");
    }

    #[test]
    fn bad_bat() {
        run_test(&["bad", "bat"], "ba[dt]");
    }

    #[test]
    fn a_b_a() {
        run_test(&["a", "b", "a"], "[ab]");
    }

    #[test]
    fn escape() {
        run_test(&["a.b"], "a\\.b");
    }

    #[test]
    fn empty() {
        run_test(&[], "");
    }

    #[test]
    fn a_ab_abc() {
        run_test(&["a", "ab", "abc"], "a");
    }

    #[test]
    fn car_cat_can_bar_bat() {
        run_test(&["car", "cat", "can", "bar", "bat"], "(ba[rt]|ca[nrt])");
    }
    #[test]
    fn surrogate_pair() {
        run_test(&["𠮟", "𠮷"], "[𠮟𠮷]");
    }
}