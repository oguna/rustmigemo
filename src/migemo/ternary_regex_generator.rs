use super::regex_generator::{RegexOperator};

#[derive(Debug)]
pub struct TernaryRegexNode {
	pub code: u16,
	pub child: Option<Box<TernaryRegexNode>>,
	pub left: Option<Box<TernaryRegexNode>>,
	pub right: Option<Box<TernaryRegexNode>>,
    pub level: usize,
}

#[derive(Debug)]
pub struct TernaryRegexGenerator {
    pub root: Option<Box<TernaryRegexNode>>,
    //pub unused: Option<Box<TernaryRegexNode>>,
}

fn skew(t: Option<Box<TernaryRegexNode>>) -> Option<Box<TernaryRegexNode>> {
    match t {
        None => {
            return None;
        },
        Some(mut t1) => {
            let level = t1.level;
            if t1.left.is_none() {
                return Some(t1);
            }
            if t1.level == level {
                let mut l = t1.left.take().unwrap();
                let b = l.right.take();
                t1.left = b;
                l.right = Some(t1);
                return Some(l);
            } else {
                return Some(t1);
            }
        }
    }
}

fn split(t: Option<Box<TernaryRegexNode>>) -> Option<Box<TernaryRegexNode>> {
    match t {
        None => {
            return None;
        },
        Some(mut tt) => {
            if tt.as_ref().right.is_none() || tt.right.as_ref().unwrap().right.is_none() {
                return Some(tt);
            } else if tt.level == tt.right.as_ref().unwrap().right.as_ref().unwrap().level {
                let mut r = tt.right.take().unwrap();
                tt.right = std::mem::replace(&mut r.left, None);
                r.left = Some(tt);
                r.level = r.level + 1;
                return Some(r);
            } else {
                return Some(tt);
            }
        }
    }
}

fn insert(word: Vec<u16>, offset: usize, t: Option<Box<TernaryRegexNode>>) -> Option<Box<TernaryRegexNode>> {
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

fn traverse_siblings(node: &Option<Box<TernaryRegexNode>>) -> Vec<&Box<TernaryRegexNode>> {
    match node {
        None => {
            vec![]
        },
        Some(ref n) => {
            let mut children = traverse_siblings(&n.left);
            children.push(n);
            children.append(&mut traverse_siblings(&n.right));
            children
        }
    }
}

fn is_characters_to_escape(c: u16) -> bool {
    let u = [9223494151364935680u64, 4035225268137230336u64];
    if c < 128 {
        return (u[(c as usize)/64]>>((c as u64)%64))&1 == 1;
    } else {
        return false;
    }
}

fn generate(node: &Option<Box<TernaryRegexNode>>, buffer: &mut Vec<u16>, op: &RegexOperator) {
    let mut brother = 0;
    let mut haschild = 0;
    let siblings = traverse_siblings(node);
    for n in &siblings {
        brother += 1;
        if n.child.is_some() {
            haschild += 1;
        }
    }
    let nochild = brother - haschild;
    if brother > 1 && haschild > 0 {
        buffer.push(b'(' as u16);
    }
    if nochild > 0 {
        if nochild > 1 {
            buffer.push(b'[' as u16);
        }
        for n in &siblings {
            if n.child.is_some() {
                continue;
            }
            if is_characters_to_escape(n.code) {
                buffer.push(b'\\' as u16);
            }
            buffer.push(n.code);
        }
        if nochild > 1 {
            buffer.push(b']' as u16);
        }
    }
    if haschild > 0 {
        if nochild > 0 {
            buffer.push(b'|' as u16);
        }
        for n in &siblings {
            if n.child.is_some() {
                if is_characters_to_escape(n.code) {
                    buffer.push(b'\\' as u16);
                }
                buffer.push(n.code);
                generate(&n.child, buffer, op);
                if haschild > 1 {
                    buffer.push(b'|' as u16);
                }
            }
        }
        if haschild > 1 {
            buffer.pop();
        }
    }
    if brother > 1 && haschild > 0 {
        buffer.push(b')' as u16);
    }
}

impl TernaryRegexGenerator {
    pub fn new() -> TernaryRegexGenerator {
        return TernaryRegexGenerator {
            root: None,
        }
    }

    pub fn add(&mut self, word: &Vec<u16>) {
        if word.len() == 0 {
            return;
        }
        self.root = insert(word.to_vec(), 0, ::std::mem::replace(&mut self.root, None));
    }

    pub fn generate(&self, op: &RegexOperator) -> String {
        if self.root.is_none() {
            return String::new();
        } else {
            let mut buffer = Vec::<u16>::new();
            generate(& self.root, &mut buffer, op);
            return String::from_utf16_lossy(&buffer);
        }
    }
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bad_dad() {
		let mut rxgen = TernaryRegexGenerator::new();
		let rxop = RegexOperator::Default;
		let bad: Vec<u16> = "bad".encode_utf16().collect();
		let dad: Vec<u16> = "dad".encode_utf16().collect();
		rxgen.add(&bad);
		rxgen.add(&dad);
		let actual = rxgen.generate(&rxop);
		let expected = "(bad|dad)";
		assert_eq!(actual, expected);
	}

	#[test]
	fn dad_bat() {
		let mut rxgen = TernaryRegexGenerator::new();
        let rxop = RegexOperator::Default;
		let bad: Vec<u16> = "bad".encode_utf16().collect();
		let bat: Vec<u16> = "bat".encode_utf16().collect();
		rxgen.add(&bad);
		rxgen.add(&bat);
		let actual = rxgen.generate(&rxop);
		let expected = "ba[dt]";
		assert_eq!(actual, expected);
	}

	#[test]
	fn a_b_a() {
		let mut rxgen = TernaryRegexGenerator::new();
		let rxop = RegexOperator::Default;
		let a1: Vec<u16> = "a".encode_utf16().collect();
		let b: Vec<u16> = "b".encode_utf16().collect();
		let a2: Vec<u16> = "a".encode_utf16().collect();
		rxgen.add(&a1);
		rxgen.add(&b);
		rxgen.add(&a2);
		let actual = rxgen.generate(&rxop);
		let expected = "[ab]";
		assert_eq!(actual, expected);
	}

	#[test]
	fn escape() {
		let mut rxgen = TernaryRegexGenerator::new();
		let rxop = RegexOperator::Default;
		let a_b: Vec<u16> = "a.b".encode_utf16().collect();
		rxgen.add(&a_b);
		let actual = rxgen.generate(&rxop);
		let expected = "a\\.b";
		assert_eq!(actual, expected);
	}
}
