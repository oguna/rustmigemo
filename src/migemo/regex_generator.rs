use std::char::{decode_utf16, REPLACEMENT_CHARACTER};

#[derive(Debug)]
pub struct RegexNode {
	pub code: char,
	pub child: Option<Box<RegexNode>>,
	pub next: Option<Box<RegexNode>>,
}

pub enum RegexOperator {
	Default,
	Vim,
	Emacs,
	VimNonNewline,
	EmacsNonNewline,
	User {
	    or: String,
	    begin_group: String,
	    end_group: String,
	    begin_class: String,
	    end_class: String,
	    newline: String,
	},
}

#[derive(Debug)]
pub struct RegexOperatorDetail {
	pub or: String,
	pub begin_group: String,
	pub end_group: String,
	pub begin_class: String,
	pub end_class: String,
	pub newline: String,
}

impl RegexOperatorDetail {
	pub fn get_regex_operator_detail(rxop: &RegexOperator) -> RegexOperatorDetail {
		return match rxop {
			RegexOperator::Default => RegexOperatorDetail {
	   				or: "|".to_string(),
	   				begin_group: "(".to_string(),
	   				end_group: ")".to_string(),
	   				begin_class: "[".to_string(),
	   				end_class: "]".to_string(),
	   				newline: String::new(),
				},
			RegexOperator::Vim => RegexOperatorDetail {
	   				or: "\\|".to_string(),
	   				begin_group: "\\%(".to_string(),
	   				end_group: "\\)".to_string(),
	   				begin_class: "[".to_string(),
	   				end_class: "]".to_string(),
	   				newline: "\\_s*".to_string(),
				},
			RegexOperator::VimNonNewline => RegexOperatorDetail {
	   				or: "\\|".to_string(),
	   				begin_group: "\\%(".to_string(),
	   				end_group: "\\)".to_string(),
	   				begin_class: "[".to_string(),
	   				end_class: "]".to_string(),
	   				newline: String::new(),
				},
			RegexOperator::Emacs => RegexOperatorDetail {
	   				or: "\\|".to_string(),
	   				begin_group: "\\(".to_string(),
	   				end_group: "\\)".to_string(),
	   				begin_class: "[".to_string(),
	   				end_class: "]".to_string(),
	   				newline: "\\_s-*".to_string(),
				},
			RegexOperator::EmacsNonNewline => RegexOperatorDetail {
	   				or: "\\|".to_string(),
	   				begin_group: "\\(".to_string(),
	   				end_group: "\\)".to_string(),
	   				begin_class: "[".to_string(),
	   				end_class: "]".to_string(),
	   				newline: String::new(),
				},
				RegexOperator::User {
					or,
					begin_group,
					end_group,
					begin_class,
					end_class,
					newline} => RegexOperatorDetail {
						or: or.clone(),
						begin_group: begin_group.clone(),
						end_group: end_group.clone(),
						begin_class: begin_class.clone(),
						end_class: end_class.clone(),
						newline: newline.clone(),
				},
		}
	}
}

#[derive(Debug)]
pub struct RegexGenerator {
	pub root: Option<Box<RegexNode>>,
}

impl RegexGenerator {
	pub fn add(&mut self, word: &Vec<u16>) {
		if word.len() == 0 {
			return;
		}
		let utf32word: Vec<char> = decode_utf16(word.iter().cloned())
                   .map(|r| r.unwrap_or(REPLACEMENT_CHARACTER))
                   .collect();
		self.root = RegexGenerator::_add(::std::mem::replace(&mut self.root, None), &utf32word, 0);
	}

	fn _add(
		node: Option<Box<RegexNode>>,
		word: &Vec<char>,
		offset: usize,
	) -> Option<Box<RegexNode>> {
		if node.is_none() {
			if offset >= word.len() {
				return None;
			}
			let child = if offset < word.len() - 1 {
				RegexGenerator::_add(None, word, offset + 1)
			} else {
				None
			};
			return Some(Box::new(RegexNode {
				code: word[offset],
				child: child,
				next: None,
			}));
		}
		let code = word[offset];
		// 最初のノードがcodeより大きければ、リストの先頭に挿入
		if code < node.as_ref().unwrap().code {
			return Some(Box::new(RegexNode {
				code: code,
				child: RegexGenerator::_add(None, word, offset + 1),
				next: ::std::mem::replace(&mut node.unwrap().next, None),
			}));
		} else {
			fn find_le_node(
				node: &mut Box<RegexNode>,
				code: char,
			) -> &mut Box<RegexNode> {
				if node.next.is_some() && node.next.as_ref().unwrap().code <= code {
					if let Some(ref mut _n) = node.next {
						return find_le_node(_n, code);
					} else {
						panic!()
					}
				} else {
					return node;
				}
			}
			let mut _node = node.unwrap();
			let another_node = find_le_node(&mut _node, code);
			if another_node.as_ref().code == code {
				if another_node.as_ref().child.is_none() {
					return Some(_node);
				} else {
					if word.len() == offset + 1 {
						another_node.child = None;
					} else {
						another_node.child = RegexGenerator::_add(::std::mem::replace(&mut another_node.child, None), word, offset + 1)
					}
					return Some(_node);
				}
			} else {
				let mut new_node =Box::new(RegexNode {
					code: code,
					child: None,
					next: ::std::mem::replace(&mut another_node.next, None),
				});
				new_node.child = if offset + 1 == word.len() {
					None
				} else {
					RegexGenerator::_add(None, word, offset + 1)
				};
				another_node.next = Some(new_node);
				return Some(_node);
			}
		}
	}

	pub fn generate(&self, operator: &RegexOperator) -> String {
		return match &self.root {
			Some(_) => {
				let mut string: String = String::new();
				let operator_detail = RegexOperatorDetail::get_regex_operator_detail(operator);
				self.generate_stub(&self.root, &operator_detail, &mut string);
				string
			},
			None => "".to_string(),
		};
	}

	fn generate_stub(&self, node: &Option<Box<RegexNode>>, operator: &RegexOperatorDetail, buf: &mut String) {
		let mut escape_characters: Vec<char> =
			"\\.[]{}()*+-?^$|".chars().collect();
		escape_characters.sort();
		let mut brother = 1;
		let mut haschild = 0;
		let mut tmp = node;
		while tmp.is_some() {
			let tmp_unwrap = tmp.as_ref().unwrap();
			if tmp_unwrap.next.is_some() {
				brother = brother + 1;
			}
			if tmp_unwrap.child.is_some() {
				haschild = haschild + 1;
			}
			tmp = &tmp_unwrap.next;
		}
		let nochild = brother - haschild;

		if brother > 1 && haschild > 0 {
			buf.push_str(&operator.begin_group);
		}
		if nochild > 0 {
			if nochild > 1 {
				buf.push_str(&operator.begin_class);
			}
			let mut tmp = node;
			while tmp.is_some() {
				let tmp_unwrap = tmp.as_ref().unwrap();
				if tmp_unwrap.child.is_some() {
					tmp = &tmp_unwrap.next;
					continue;
				}
				if escape_characters.binary_search(&tmp_unwrap.code).is_ok() {
					buf.push('\\');
				}
				buf.push(::std::char::from_u32(tmp_unwrap.code as u32).unwrap());
				tmp = &tmp_unwrap.next;
			}
			if nochild > 1 {
				buf.push_str(&operator.end_class);
			}
		}

		if haschild > 0 {
			if nochild > 0 {
				buf.push_str(&operator.or);
			}
			let mut tmp = node;
			while tmp.as_ref().unwrap().child.is_none() {
				tmp = &tmp.as_ref().unwrap().next;
			}
			loop {
				if escape_characters
					.binary_search(&tmp.as_ref().unwrap().code)
					.is_ok()
				{
					buf.push('\\');
				}
				buf.push(tmp.as_ref().unwrap().code);
				if operator.newline.len() > 0 {
					buf.push_str(&operator.newline);
				}
				self.generate_stub(&tmp.as_ref().unwrap().child, operator, buf);
				tmp = &tmp.as_ref().unwrap().next;
				while tmp.is_some() && tmp.as_ref().unwrap().child.is_none() {
					tmp = &tmp.as_ref().unwrap().next;
				}
				if tmp.is_none() {
					break;
				}
				if haschild > 1 {
					buf.push_str(&operator.or);
				}
			}
		}
		if brother > 1 && haschild > 0 {
			buf.push_str(&operator.end_group);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bad_dad() {
		let mut rxgen = RegexGenerator {
			root: None,
		};
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
		let mut rxgen = RegexGenerator {
			root: None,
		};
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
		let mut rxgen = RegexGenerator {
			root: None,
		};
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
		let mut rxgen = RegexGenerator {
			root: None,
		};
		let rxop = RegexOperator::Default;
		let a_b: Vec<u16> = "a.b".encode_utf16().collect();
		rxgen.add(&a_b);
		let actual = rxgen.generate(&rxop);
		let expected = "a\\.b";
		assert_eq!(actual, expected);
	}

	#[test]
	fn surrogate_pair() {
		let mut rxgen = RegexGenerator {
			root: None,
		};
		let rxop = RegexOperator::Default;
		let a: Vec<u16> = "𠮟".encode_utf16().collect();
		let b: Vec<u16> = "𠮷".encode_utf16().collect();
		rxgen.add(&a);
		rxgen.add(&b);
		let actual = rxgen.generate(&rxop);
		let expected = "[𠮟𠮷]";
		assert_eq!(actual, expected);
	}
}
