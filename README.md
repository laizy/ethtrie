## ethtrie

Rust implementation of the Merkle-Patricia Trie.

The implementation is forked from [cita-trie](https://crates.io/crates/cita_trie) to support no_std.

## Features

- Support `no_std`
- Implementation of the Modified Patricia Tree
- Custom storage interface

## Example

```rust
use ethtrie::{TrieDBMut, MemoryDB, keccak256};
fn main() {
    let mut memdb = MemoryDB::new(true);
    let key = keccak256(b"test-key");
    let value = b"test-value";
    let root = {
        let mut trie = TrieDBMut::new(&mut memdb);
        trie.insert(&key, value.to_vec()).unwrap();
        let v = trie.get(&key).unwrap();
        assert_eq!(Some(value.to_vec()), v);
        trie.root().unwrap()
    };
    
    let mut trie = TrieDBMut::from(&mut memdb, root).unwrap();
    let exists = trie.contains(&key).unwrap();
    assert_eq!(exists, true);
    let removed = trie.remove(&key).unwrap();
    assert_eq!(removed, true);
    let new_root = trie.root().unwrap();
    println!("new root = {:?}", new_root);
}
```

## Benchmark

```sh
> cargo bench
insert one              time:   [779.96 ns 794.04 ns 807.55 ns]
insert 1k               time:   [278.41 us 279.84 us 281.50 us]
insert 10k              time:   [3.2367 ms 3.2616 ms 3.2883 ms]
get based 10k           time:   [256.94 ns 259.88 ns 262.76 ns]
remove 1k               time:   [150.92 us 152.49 us 154.35 us]
remove 10k              time:   [1.5751 ms 1.5893 ms 1.6062 ms]
```