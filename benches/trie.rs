use criterion::{criterion_group, criterion_main, Criterion};

use ethtrie::PatriciaTrieMut;
use ethtrie::{keccak256, MemoryDB};

fn insert_worse_case_benchmark(c: &mut Criterion) {
    c.bench_function("insert one", |b| {
        let mut memdb = MemoryDB::new(false);
        let mut trie = PatriciaTrieMut::new(&mut memdb);

        let (keys, values) = random_data(1);
        b.iter(|| trie.insert(&keys[0], values[0].clone()).unwrap())
    });

    c.bench_function("insert 1k", |b| {
        let mut memdb = MemoryDB::new(false);
        let mut trie = PatriciaTrieMut::new(&mut memdb);

        let (keys, values) = random_data(1000);
        b.iter(|| {
            for i in 0..keys.len() {
                trie.insert(&keys[i], values[i].clone()).unwrap()
            }
        });
    });

    c.bench_function("insert 10k", |b| {
        let mut memdb = MemoryDB::new(false);
        let mut trie = PatriciaTrieMut::new(&mut memdb);

        let (keys, values) = random_data(10000);
        b.iter(|| {
            for i in 0..keys.len() {
                trie.insert(&keys[i], values[i].clone()).unwrap()
            }
        });
    });

    c.bench_function("get based 10k", |b| {
        let mut memdb = MemoryDB::new(false);
        let mut trie = PatriciaTrieMut::new(&mut memdb);

        let (keys, values) = random_data(10000);
        for i in 0..keys.len() {
            trie.insert(&keys[i], values[i].clone()).unwrap()
        }

        b.iter(|| {
            let key = trie.get(&keys[7777]).unwrap();
            assert_ne!(key, None);
        });
    });

    c.bench_function("remove 1k", |b| {
        let mut memdb = MemoryDB::new(false);
        let mut trie = PatriciaTrieMut::new(&mut memdb);

        let (keys, values) = random_data(1000);
        for i in 0..keys.len() {
            trie.insert(&keys[i], values[i].clone()).unwrap()
        }

        b.iter(|| {
            for key in keys.iter() {
                trie.remove(key).unwrap();
            }
        });
    });

    c.bench_function("remove 10k", |b| {
        let mut memdb = MemoryDB::new(false);
        let mut trie = PatriciaTrieMut::new(&mut memdb);

        let (keys, values) = random_data(10000);
        for i in 0..keys.len() {
            trie.insert(&keys[i], values[i].clone()).unwrap()
        }

        b.iter(|| {
            for key in keys.iter() {
                trie.remove(key).unwrap();
            }
        });
    });
}

fn random_data(n: usize) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
    let mut keys = Vec::with_capacity(n);
    let mut values = Vec::with_capacity(n);
    for i in 0..n {
        let key = keccak256(i.to_le_bytes().as_slice()).as_bytes().to_vec();
        let value = key.clone();
        keys.push(key);
        values.push(value);
    }

    (keys, values)
}

criterion_group!(benches, insert_worse_case_benchmark);
criterion_main!(benches);
