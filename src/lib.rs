#[doc = include_str!("../readme.md")]

use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct TreeNode<'a, T> where T: Clone + Default {
	depth: i32,
	subtree_signatures: Vec<SignatureInfo<T>>,
	choices:Vec<Option<&'a TreeNode<'a, T>>>,
	term: Vec<SignatureInfo<T>>,
}

impl<'a, T> Default for TreeNode<'a, T> where T: Clone + Default {
	fn default() -> Self {
		TreeNode {
			depth: 0,
			subtree_signatures: Vec::new(),
			choices: vec![None; 256],
			term: Vec::new()
		}
	}
}

#[derive(Clone, Debug)]
pub struct SignatureInfo<T> where T: Clone + Default {
	bytes: Vec<u8>,
	masks: Vec<u8>,
	object: T
}

#[derive(Clone, Debug)]
pub struct SignatureDecisionTree<'a, T> where T: Clone + Default {
	base_node: TreeNode<'a, T>,
	sigs_dup: HashMap<Vec<u8>, bool>
}

impl<'a, T> SignatureDecisionTree<'a, T> where T: Clone + Default {
	pub fn new() -> Self {
		SignatureDecisionTree {
			base_node: TreeNode::default(),
			sigs_dup: HashMap::new()
		}
	}

	fn add_choice(&mut self, signature_info: SignatureInfo<T>, tree_node: TreeNode<'a, T>) {
		let mut node_info_list = vec![(tree_node, signature_info)];
		// Workaround to avoid recursion
		while !node_info_list.is_empty() {
			let (mut node, sig_info) = node_info_list.pop().unwrap();
			let (depth, mut sigs, choices, mut term) = (node.depth, node.subtree_signatures, node.choices, node.term);
			let (bytes, _, _) = (&sig_info.bytes, &sig_info.masks, &sig_info.object);
			let siglen = sigs.len();
			if bytes.len() as i32 > depth {
				sigs.push(sig_info.clone());
			} else {
				term.push(sig_info.clone());
				continue;
			}
			// If one sig is [85, 139, 236] and another is [85, 139, 236, 232, 144], then
			// we're gonna panic without this check
			if siglen == 0 {
				// we just don't want the "else" here, if we're the only
				// one on this node, just let it ride.
				break
			} else if siglen == 1 {
				// If it has one already, we *both* need to add another level
				// (because if it is the only one, it thought it was last choice)
				for sig in sigs.iter() {
					let ch_val = sig.bytes[depth as usize];
					let nn_node = self.get_node(depth, choices.clone(), ch_val as i32);
					node_info_list.push((nn_node.cloned().unwrap(), sig.clone()));
				}
			} else {
				// This is already a choice node, keep on choosing...
				let ch_val = bytes[depth as usize];
				let nn_node = self.get_node(depth, choices.clone(), ch_val as i32);
				node_info_list.push((nn_node.cloned().unwrap(), sig_info));
			}
		}
	}

	/// Chose, (and or initialize) a sub node.
	fn get_node(&self, depth: i32, mut choices: Vec<Option<&'a TreeNode<'a, T>>>, choice: i32) -> Option<&'a TreeNode<'a, T>> {
		let nn_node = choices[choice as usize];
		if nn_node.is_none() {
			let nn_node = TreeNode{
				depth: depth + 1,
				..Default::default()
			};
			choices[choice as usize] = Some(&nn_node);
		}
		nn_node
	}

	/// Add a signature to the search tree.  If masks goes unspecified, it will be
	/// assumed to be all ones (\\xff * len(bytes)).
	/// 
	/// Additionally, you may specify "val" as the object to get back with
	/// getSignature().
	pub fn add_signature(&mut self, bytes: Vec<u8>, masks: Option<Vec<u8>>, val: Option<T>) {
		let masks = masks.unwrap_or(vec![0xff; bytes.len()]);
		let val = val.unwrap_or_default();
		// Detect and skip duplicate additions...
		let byte_key = vec![bytes.clone(), masks.clone()].concat();
		if self.sigs_dup.contains_key(&byte_key) {
			return
		}
		self.sigs_dup.insert(byte_key, true);
		let sig_info = SignatureInfo {
			bytes,
			masks,
			object: val
		};
		self.add_choice(sig_info, self.base_node.clone());
	}

	pub fn is_signature(&self, bytes: Vec<u8>, offset: Option<i32>) -> bool {
		self.get_signature(bytes, offset).is_some()
	}

	pub fn get_signature(&self, bytes: Vec<u8>, offset: Option<i32>) -> Option<T> {
		let offset = offset.unwrap_or_default();
		let mut matches = vec![];
		let node = self.base_node.clone();
		loop {
			let (depth, sigs, choices, term) = (&node.depth, &node.subtree_signatures, &node.choices, &node.term);
			matches.append(&mut term.clone());
			// Once we get down to one sig, there are no more branches,
			// just check the byte sequence.
			if sigs.len() == 1 {
				let (sbytes, smasks, _) = (&sigs[0].bytes, &sigs[0].masks, &sigs[0].object);
				let mut is_match = true;
				for i in (*depth as usize)..sbytes.len() {
					let real_off = offset + i as i32;
					// We still have pieces of the signature left, but we're out of bytes
					if real_off >= bytes.len() as i32 {
						is_match = false;
						break
					}
					let masked = bytes[real_off as usize] & smasks[i];
					if masked != sbytes[i] {
						is_match = false;
						break
					}
				}
				if is_match {
					matches.push(sigs[0].clone());
				}
				break;
			}
			// There are still more choices to make, keep on truckin'
			let mut node = None;
			for sig in sigs.iter() {
				let (sbytes, smasks, _) = (&sig.bytes, &sig.masks, &sig.object);
				if (offset + *depth) >= bytes.len() as i32 {
					continue
				}
				// We've reached the end of the signature, Just mask the rest
				let masked = bytes[(offset + *depth) as usize] & smasks[*depth as usize];
				if masked == sbytes[*depth as usize] {
					// FIXME: Find the *best* winner! Because of masking.
					node = Some(choices[masked as usize]);
					break
				}
			}
			// We failed to make our next choice
			if node.is_none() {
				break
			}
		}
		return if matches.is_empty() {
			None
		} else {
			matches.sort_by(|a, b| a.bytes.len().cmp(&b.bytes.len()));
			matches.first().map(|x| x.object.clone())
		}
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_signature_subset() {
		// signature_base = b'\x55\xe9\xd8\x01\xfe\xff\x32\x77\x89\x4f\x55'
		//         sigtree = envi.bytesig.SignatureTree()
		//         sigtree.addSignature(signature_base)
		//
		//         sigtree.getSignature(b'\x55\xe9')
		//         sigtree.addSignature(signature_base[:7], val=signature_base[:7])
		//         sigtree.addSignature(signature_base[:4], val=signature_base[:4])
		//         sigtree.addSignature(signature_base + b'\xfe\x38', val=signature_base + b'\xfe\x38')
		//
		//         self.assertTrue(sigtree.getSignature(b'\x55\xe9\xd8\x01\xfe\xff\x32\x00\x99\x36\x5f\x21\xfd') == signature_base[:7])
		//         self.assertTrue(sigtree.getSignature(b'\x55\xe9\xd8\x01\xfe\xff\x32') == signature_base[:7])
		//         self.assertTrue(sigtree.getSignature(b'\x55\xe9\xd8\x01\xfe\x00') == signature_base[:4])
		//         self.assertTrue(sigtree.getSignature(b'\x55') is None)
		let signature_base = vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32, 0x77, 0x89, 0x4f, 0x55];
		let mut tree = super::SignatureDecisionTree::new();
		tree.add_signature(signature_base.clone(), None, None);
		tree.get_signature(vec![0x55, 0xe9], None);
		tree.add_signature(signature_base.clone().into_iter().take(7).collect(), None, Some(signature_base.clone().into_iter().take(7).collect()));
		tree.add_signature(signature_base.clone().into_iter().take(4).collect(), None, Some(signature_base.clone().into_iter().take(4).collect()));
		tree.add_signature([signature_base.clone(), vec![0xfe, 0x38]].concat(), None, Some([signature_base.clone(), vec![0xfe, 0x38]].concat()));
		assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32, 0x00, 0x99, 0x36, 0x5f, 0x21, 0xfd], None), Some(signature_base.clone().into_iter().take(7).collect()));
		assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0xff, 0x32], None), Some(signature_base.clone().into_iter().take(7).collect()));
		assert_eq!(tree.get_signature(vec![0x55, 0xe9, 0xd8, 0x01, 0xfe, 0x00], None), Some(signature_base.clone().into_iter().take(4).collect()));
		assert_eq!(tree.get_signature(vec![0x55], None), None);
	}
}
