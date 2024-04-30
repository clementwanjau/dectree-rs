## Dectree

[![Build](https://github.com/clementwanjau/dectree-rs/actions/workflows/build.yml/badge.svg)](https://github.com/clementwanjau/dectree-rs/actions/workflows/build.yml)

A byte and mask based decision engine for creating byte
sequences (and potential comparison masks) for general purpose
signature matching implemented in pure rust.

The decision tree uses a lot of RAM but is very fast.

Example:
```rust
use dectree::SignatureDecisionTree;

fn main() {
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

```

### Authors
- Clement Wanjau <clementwanjau@gmail.com>

