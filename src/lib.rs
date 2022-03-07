#![no_std]

//! ## Usage
//!
//! ```rust
//! use ethtrie::{TrieDB, MemoryDB, keccak256};
//! fn main() {
//!     let mut memdb = MemoryDB::new(true);
//!
//!     let key = keccak256(b"test-key");
//!     let value = b"test-value";
//!
//!     let root = {
//!         let mut trie = TrieDB::new(&mut memdb);
//!         trie.insert(&key, value.to_vec()).unwrap();
//!
//!         let v = trie.get(&key).unwrap();
//!         assert_eq!(Some(value.to_vec()), v);
//!         trie.root().unwrap()
//!     };
//!
//!     let mut trie = TrieDB::from(&mut memdb, root).unwrap();
//!     let exists = trie.contains(&key).unwrap();
//!     assert_eq!(exists, true);
//!     let removed = trie.remove(&key).unwrap();
//!     assert_eq!(removed, true);
//!     let new_root = trie.root().unwrap();
//!     println!("new root = {:?}", new_root);
//! }
//! ```

mod nibbles;
mod node;

mod db;
mod errors;
mod eth;
mod hasher;
mod trie;

pub use db::{HashDB, MemoryDB};
pub use errors::TrieError;
pub use eth::{SecTrieDB, TrieDB};
pub use hasher::keccak256;
pub use trie::PatriciaTrie;

pub use ethereum_types::H256;
