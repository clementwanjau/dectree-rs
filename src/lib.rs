#[doc = include_str!("../readme.md")]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Represents a reference counted reference to a `RefCell<TreeNode<T>>`. This is used for nodes that 
/// need to be mutated but are shared between multiple references.

type RcRefCellTreeNode<T> = Rc<RefCell<TreeNode<T>>>;

/// Represents a node in the decision tree. This is a recursive structure that can be used to represent
/// a decision tree where each node is a choice and the leaf nodes are the final decision.
#[derive(Clone, Debug)]
struct TreeNode<T> where T: Clone + Default {
	/// The depth of the node in the tree.
	depth: i32,
	/// The signatures that are valid at this node.
	subtree_signatures: Vec<SignatureInfo<T>>,
	/// The choices that can be made at this node.
	choices:Vec<Option<RcRefCellTreeNode<T>>>,
	/// The final decision at this node.
	term: Vec<SignatureInfo<T>>,
}

impl<T> Default for TreeNode<T> where T: Clone + Default {
	fn default() -> Self {
		TreeNode {
			depth: 0,
			subtree_signatures: Vec::new(),
			choices: vec![None; 256],
			term: Vec::new()
		}
	}
}

/// Represents signature information. This is used to store the signature bytes, masks, and the object
/// that is associated with the signature.
#[derive(Clone, Debug)]
struct SignatureInfo<T> where T: Clone + Default {
	bytes: Vec<u8>,
	masks: Vec<u8>,
	object: T
}

/// Represents a decision tree that can be used to search for signatures. This is a tree structure that
/// can be used to search for signatures in a binary blob. The tree is built by adding signatures to the
/// tree and then searching for them.
/// ```rust
/// use dectree_rs::SignatureDecisionTree;
/// 
/// let mut tree = SignatureDecisionTree::new();
/// tree.add_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32, 0x77, 0x89, 0x4f, 0x55], None, None);
/// tree.add_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32], None, None);
/// tree.add_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0x00], None, None);
/// assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32, 0x00, 0x99, 0x36, 0x5f, 0x21, 0xfd], None), Some(()));
/// assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32], None), Some(()));
/// assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0x00], None), Some(()));
/// assert_eq!(tree.get_signature(vec![0x55], None), None);
/// ```
#[derive(Clone, Debug, Default)]
pub struct SignatureDecisionTree<T> where T: Clone + Default {
	base_node: RcRefCellTreeNode<T>,
	sigs_dup: HashMap<Vec<u8>, bool>
}

impl<T> SignatureDecisionTree<T> where T: Clone + Default {
	
	/// Create a new `SignatureDecisionTree`.
	pub fn new() -> Self {
		SignatureDecisionTree::default()
	}

	/// Add a choice to the search tree.
	fn add_choice(&mut self, signature_info: SignatureInfo<T>, tree_node: Rc<RefCell<TreeNode<T>>>) {
		let mut node_info_list = vec![(tree_node, signature_info)];
		// Workaround to avoid recursion
		while let Some((node, sig_info)) = node_info_list.pop() {
			let mut borrowed_node = node.borrow_mut();
			let (depth, mut sigs, choices, mut term) = (borrowed_node.depth, borrowed_node.subtree_signatures.clone(), borrowed_node.choices.clone(), borrowed_node.term.clone());
			let bytes = &sig_info.bytes;
			let siglen = sigs.len();
			if bytes.len() as i32 > depth {
				sigs.push(sig_info.clone());
				*borrowed_node = TreeNode {
					depth: borrowed_node.depth,
					subtree_signatures: sigs.clone(),
					choices: borrowed_node.choices.clone(),
					term: borrowed_node.term.clone()
				};
			} else {
				term.push(sig_info.clone());
				*borrowed_node = TreeNode {
					depth: borrowed_node.depth,
					subtree_signatures: borrowed_node.subtree_signatures.clone(),
					choices: borrowed_node.choices.clone(),
					term
				};
				continue;
			}
			let choices = Rc::new(RefCell::new(choices));
			// If one sig is [85, 139, 236] and another is [85, 139, 236, 232, 144], then
			// we're gonna panic without this check
			if siglen == 0 {
				// If it has no sigs, we need to add a level
				// modify the next node
				continue;
			} else if siglen == 1 {
				// If it has one already, we *both* need to add another level
				// (because if it is the only one, it thought it was last choice)
				for sig in sigs.iter() {
					let ch_val = sig.bytes[depth as usize];
					let nn_node = self.get_node(depth, choices.clone(), ch_val as i32);
					*borrowed_node = TreeNode {
						depth: borrowed_node.depth,
						subtree_signatures: borrowed_node.subtree_signatures.clone(),
						choices: choices.borrow().clone(),
						term: borrowed_node.term.clone()
					};
					node_info_list.push((nn_node, sig.clone()));
				}
			} else {
				// This is already a choice node, keep on choosing...
				let ch_val = bytes[depth as usize];
				let nn_node = self.get_node(depth, choices.clone(), ch_val as i32);
				*borrowed_node = TreeNode {
					depth: borrowed_node.depth,
					subtree_signatures: borrowed_node.subtree_signatures.clone(),
					choices: choices.borrow().clone(),
					term: borrowed_node.term.clone()
				};
				node_info_list.push((nn_node, sig_info));
			}
		}
	}

	/// Chose, (and or initialize) a sub node.
	fn get_node(&self, depth: i32, choices: Rc<RefCell<Vec<Option<RcRefCellTreeNode<T>>>>>, choice: i32) -> RcRefCellTreeNode<T> {
		let mut borrowed_choices = choices.borrow_mut();
		let nn_node = borrowed_choices[choice as usize].clone();
		if nn_node.is_none() {
			let nn_node = TreeNode{
				depth: depth + 1,
				..Default::default()
			};
			borrowed_choices[choice as usize] = Some(Rc::new(RefCell::new(nn_node)));
		}
		let nn_node = borrowed_choices[choice as usize].as_ref().unwrap();
		nn_node.clone()
	}

	/// Add a signature to the search tree.  If masks goes unspecified, it will be
	/// assumed to be all ones `vec![0xff; bytes.len()]`.
	/// 
	/// Additionally, you may specify `val` as the object to get back with
	/// `tree.get_signature()`.
	pub fn add_signature(&mut self, bytes: Vec<u8>, masks: Option<Vec<u8>>, val: Option<T>) {
		let masks = masks.unwrap_or(vec![0xff; bytes.len()]);
		let val = val.unwrap_or_default();
		// Detect and skip duplicate additions...
		let byte_key = [bytes.clone(), masks.clone()].concat();
		if self.sigs_dup.contains_key(&byte_key) {
			return
		}
		self.sigs_dup.insert(byte_key, true);
		let sig_info = SignatureInfo {
			bytes,
			masks,
			object: val
		};
		self.add_choice(sig_info, Rc::clone(&self.base_node));
	}

	/// Check if a signature is in the search tree.
	pub fn is_signature(&self, bytes: Vec<u8>, offset: Option<i32>) -> bool {
		self.get_signature(bytes, offset).is_some()
	}

	/// Get the object associated with a signature in the search tree.
	pub fn get_signature(&self, bytes: Vec<u8>, offset: Option<i32>) -> Option<T> {
		let offset = offset.unwrap_or_default();
		let mut matches = vec![];
		let mut nn_node = Some(Rc::clone(&self.base_node));
		loop {
			if let Some(node) = nn_node {
				let node = node.borrow();
				let (depth, sigs, choices, term) = (&node.depth, &node.subtree_signatures, &node.choices, &node.term);
				matches.append(&mut term.clone());
				// Once we get down to one sig, there are no more branches,
				// just check the byte sequence.
				if sigs.len() == 1 {
					let (sbytes, smasks) = (&sigs[0].bytes, &sigs[0].masks);
					let mut is_match = true;
					for i in (*depth as usize)..sbytes.len() {
						let real_off = offset + i as i32;
						// We still have pieces of the signature left, but we're out of bytes
						if real_off >= bytes.len() as i32 {
							is_match = false;
							break;
						}
						let masked = bytes[real_off as usize] & smasks[i];
						if masked != sbytes[i] {
							is_match = false;
							break;
						}
					}
					if is_match {
						matches.push(sigs[0].clone());
					}
					break;
				}
				// There are still more choices to make, keep on truckin'
				nn_node = None;
				for sig in sigs.iter() {
					let (sbytes, smasks) = (&sig.bytes, &sig.masks);
					if (offset + *depth) >= bytes.len() as i32 {
						continue
					}
					// We've reached the end of the signature, Just mask the rest
					let masked = bytes[(offset + *depth) as usize] & smasks[*depth as usize];
					if masked == sbytes[*depth as usize] {
						// FIXME: Find the *best* winner! Because of masking.
						nn_node = choices[masked as usize].as_ref().map(Rc::clone);
						break
					}
				}
				// We failed to make our next choice
				if nn_node.is_none() {
					break
				}
			}
		}
		return if matches.is_empty() {
			None
		} else {
			matches.sort_by(|a, b| b.bytes.len().cmp(&a.bytes.len()));
			matches.first().map(|x| x.object.clone())
		}
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_signature_subset() {
		let signature_base = vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32, 0x77, 0x89, 0x4f, 0x55];
		let mut tree = super::SignatureDecisionTree::new();
		tree.add_signature(signature_base.clone(), None, None);
		tree.add_signature(signature_base.clone().into_iter().take(7).collect(), None, Some(signature_base.clone().into_iter().take(7).collect()));
		tree.add_signature(signature_base.clone().into_iter().take(4).collect(), None, Some(signature_base.clone().into_iter().take(4).collect()));
		tree.add_signature([signature_base.clone(), vec![0xfe, 0x38]].concat(), None, Some([signature_base.clone(), vec![0xfe, 0x38]].concat()));
		assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32, 0x00, 0x99, 0x36, 0x5f, 0x21, 0xfd], None), Some(signature_base.clone().into_iter().take(7).collect()));
		assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32], None), Some(signature_base.clone().into_iter().take(7).collect()));
		assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0x00], None), Some(signature_base.clone().into_iter().take(4).collect()));
		assert_eq!(tree.get_signature(vec![0x55], None), None);
	}
}
