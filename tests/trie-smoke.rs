use rand::distributions::Alphanumeric;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use std::collections::{HashMap, HashSet};

use ethereum_types::H256;
use ethtrie::{keccak256, HashDB, MemoryDB, PatriciaTrieMut};

#[test]
fn test_trie_insert() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);
    trie.insert(b"test", b"test".to_vec()).unwrap();
}

#[test]
fn test_trie_get() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);
    trie.insert(b"test", b"test".to_vec()).unwrap();
    let v = trie.get(b"test").unwrap();

    assert_eq!(Some(b"test".to_vec()), v)
}

#[test]
fn test_trie_random_insert() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);

    for _ in 0..1000 {
        let rand_str: String = thread_rng().sample_iter(&Alphanumeric).take(30).collect();
        let val = rand_str.as_bytes();
        trie.insert(val, val.to_vec()).unwrap();

        let v = trie.get(val).unwrap();
        assert_eq!(v.map(|v| v.to_vec()), Some(val.to_vec()));
    }
}

#[test]
fn test_trie_contains() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);
    trie.insert(b"test", b"test".to_vec()).unwrap();
    assert_eq!(true, trie.contains(b"test").unwrap());
    assert_eq!(false, trie.contains(b"test2").unwrap());
}

#[test]
fn test_trie_remove() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);
    trie.insert(b"test", b"test".to_vec()).unwrap();
    let removed = trie.remove(b"test").unwrap();
    assert_eq!(true, removed)
}

#[test]
fn test_trie_random_remove() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);

    for _ in 0..1000 {
        let rand_str: String = thread_rng().sample_iter(&Alphanumeric).take(30).collect();
        let val = rand_str.as_bytes();
        trie.insert(val, val.to_vec()).unwrap();

        let removed = trie.remove(val).unwrap();
        assert_eq!(true, removed);
    }
}

#[test]
fn test_trie_from_root() {
    let mut memdb = MemoryDB::new(true);
    let root = {
        let mut trie = PatriciaTrieMut::new(&mut memdb);
        trie.insert(b"test", b"test".to_vec()).unwrap();
        trie.insert(b"test1", b"test".to_vec()).unwrap();
        trie.insert(b"test2", b"test".to_vec()).unwrap();
        trie.insert(b"test23", b"test".to_vec()).unwrap();
        trie.insert(b"test33", b"test".to_vec()).unwrap();
        trie.insert(b"test44", b"test".to_vec()).unwrap();
        trie.root().unwrap()
    };

    let mut trie = PatriciaTrieMut::from(&mut memdb, root).unwrap();
    let v1 = trie.get(b"test33").unwrap();
    assert_eq!(Some(b"test".to_vec()), v1);
    let v2 = trie.get(b"test44").unwrap();
    assert_eq!(Some(b"test".to_vec()), v2);
    let root2 = trie.root().unwrap();
    assert_eq!(hex::encode(root), hex::encode(root2));
}

#[test]
fn test_trie_from_root_and_insert() {
    let mut memdb = MemoryDB::new(true);
    let root = {
        let mut trie = PatriciaTrieMut::new(&mut memdb);
        trie.insert(b"test", b"test".to_vec()).unwrap();
        trie.insert(b"test1", b"test".to_vec()).unwrap();
        trie.insert(b"test2", b"test".to_vec()).unwrap();
        trie.insert(b"test23", b"test".to_vec()).unwrap();
        trie.insert(b"test33", b"test".to_vec()).unwrap();
        trie.insert(b"test44", b"test".to_vec()).unwrap();
        trie.root().unwrap()
    };

    let mut trie = PatriciaTrieMut::from(&mut memdb, root).unwrap();
    trie.insert(b"test55", b"test55".to_vec()).unwrap();
    trie.root().unwrap();
    let v = trie.get(b"test55").unwrap();
    assert_eq!(Some(b"test55".to_vec()), v);
}

#[test]
fn test_trie_from_root_and_delete() {
    let mut memdb = MemoryDB::new(true);
    let root = {
        let mut trie = PatriciaTrieMut::new(&mut memdb);
        trie.insert(b"test", b"test".to_vec()).unwrap();
        trie.insert(b"test1", b"test".to_vec()).unwrap();
        trie.insert(b"test2", b"test".to_vec()).unwrap();
        trie.insert(b"test23", b"test".to_vec()).unwrap();
        trie.insert(b"test33", b"test".to_vec()).unwrap();
        trie.insert(b"test44", b"test".to_vec()).unwrap();
        trie.root().unwrap()
    };

    let mut trie = PatriciaTrieMut::from(&mut memdb, root).unwrap();
    let removed = trie.remove(b"test44").unwrap();
    assert_eq!(true, removed);
    let removed = trie.remove(b"test33").unwrap();
    assert_eq!(true, removed);
    let removed = trie.remove(b"test23").unwrap();
    assert_eq!(true, removed);
}

#[test]
fn test_multiple_trie_roots() {
    let k0: H256 = H256::zero();
    let k1: H256 = H256::from_low_u64_be(1);
    let v: H256 = H256::from_low_u64_be(0x1234);

    let root1 = {
        let mut memdb = MemoryDB::new(true);
        let mut trie = PatriciaTrieMut::new(&mut memdb);
        trie.insert(k0.as_bytes(), v.as_bytes().to_vec()).unwrap();
        trie.root().unwrap()
    };

    let root2 = {
        let mut memdb = MemoryDB::new(true);
        let mut trie = PatriciaTrieMut::new(&mut memdb);
        trie.insert(k0.as_bytes(), v.as_bytes().to_vec()).unwrap();
        trie.insert(k1.as_bytes(), v.as_bytes().to_vec()).unwrap();
        trie.root().unwrap();
        trie.remove(k1.as_ref()).unwrap();
        trie.root().unwrap()
    };

    let root3 = {
        let mut memdb = MemoryDB::new(true);
        let root = {
            let mut trie1 = PatriciaTrieMut::new(&mut memdb);
            trie1.insert(k0.as_bytes(), v.as_bytes().to_vec()).unwrap();
            trie1.insert(k1.as_bytes(), v.as_bytes().to_vec()).unwrap();
            trie1.root().unwrap();
            trie1.root().unwrap()
        };
        let mut trie2 = PatriciaTrieMut::from(&mut memdb, root).unwrap();
        trie2.remove(&k1.as_bytes()).unwrap();
        trie2.root().unwrap()
    };

    assert_eq!(root1, root2);
    assert_eq!(root2, root3);
}

#[test]
fn test_delete_stale_keys_with_random_insert_and_delete() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);

    let mut rng = rand::thread_rng();
    let mut keys = Vec::new();
    for _ in 0..100 {
        let random_bytes: Vec<u8> = (0..rng.gen_range(2, 30u8))
            .map(|_| rand::random::<u8>())
            .collect();
        trie.insert(&random_bytes, random_bytes.clone()).unwrap();
        keys.push(random_bytes.clone());
    }
    trie.root().unwrap();
    let slice = &mut keys;
    slice.shuffle(&mut rng);

    for key in slice.iter() {
        trie.remove(key).unwrap();
    }
    trie.root().unwrap();

    let empty_node_key = keccak256(&rlp::NULL_RLP);
    let value = memdb.get(&empty_node_key).unwrap();
    assert_eq!(value, &rlp::NULL_RLP)
}

#[test]
fn insert_full_branch() {
    let mut memdb = MemoryDB::new(true);
    let mut trie = PatriciaTrieMut::new(&mut memdb);

    trie.insert(b"test", b"test".to_vec()).unwrap();
    trie.insert(b"test1", b"test".to_vec()).unwrap();
    trie.insert(b"test2", b"test".to_vec()).unwrap();
    trie.insert(b"test23", b"test".to_vec()).unwrap();
    trie.insert(b"test33", b"test".to_vec()).unwrap();
    trie.insert(b"test44", b"test".to_vec()).unwrap();
    trie.root().unwrap();

    let v = trie.get(b"test").unwrap();
    assert_eq!(Some(b"test".to_vec()), v);
}

#[test]
fn iterator_trie() {
    let mut memdb = MemoryDB::new(true);
    let root1;
    let mut kv = HashMap::new();
    kv.insert(b"test".to_vec(), b"test".to_vec());
    kv.insert(b"test1".to_vec(), b"test1".to_vec());
    kv.insert(b"test11".to_vec(), b"test2".to_vec());
    kv.insert(b"test14".to_vec(), b"test3".to_vec());
    kv.insert(b"test16".to_vec(), b"test4".to_vec());
    kv.insert(b"test18".to_vec(), b"test5".to_vec());
    kv.insert(b"test2".to_vec(), b"test6".to_vec());
    kv.insert(b"test23".to_vec(), b"test7".to_vec());
    kv.insert(b"test9".to_vec(), b"test8".to_vec());
    {
        let mut trie = PatriciaTrieMut::new(&mut memdb);
        let mut kv = kv.clone();
        kv.iter().for_each(|(k, v)| {
            trie.insert(k, v.clone()).unwrap();
        });
        root1 = trie.root().unwrap();

        trie.iter()
            .for_each(|(k, v)| assert_eq!(kv.remove(&k).unwrap(), v));
        assert!(kv.is_empty());
    }

    {
        let mut trie = PatriciaTrieMut::new(&mut memdb);
        let mut kv2 = HashMap::new();
        kv2.insert(b"test".to_vec(), b"test11".to_vec());
        kv2.insert(b"test1".to_vec(), b"test12".to_vec());
        kv2.insert(b"test14".to_vec(), b"test13".to_vec());
        kv2.insert(b"test22".to_vec(), b"test14".to_vec());
        kv2.insert(b"test9".to_vec(), b"test15".to_vec());
        kv2.insert(b"test16".to_vec(), b"test16".to_vec());
        kv2.insert(b"test2".to_vec(), b"test17".to_vec());
        kv2.iter().for_each(|(k, v)| {
            trie.insert(k, v.clone()).unwrap();
        });

        trie.root().unwrap();

        let mut kv_delete = HashSet::new();
        kv_delete.insert(b"test".to_vec());
        kv_delete.insert(b"test1".to_vec());
        kv_delete.insert(b"test14".to_vec());

        kv_delete.iter().for_each(|k| {
            trie.remove(&k).unwrap();
        });

        kv2.retain(|k, _| !kv_delete.contains(k));

        trie.root().unwrap();
        trie.iter()
            .for_each(|(k, v)| assert_eq!(kv2.remove(&k).unwrap(), v));
        assert!(kv2.is_empty());
    }

    let trie = PatriciaTrieMut::from(&mut memdb, root1).unwrap();
    trie.iter()
        .for_each(|(k, v)| assert_eq!(kv.remove(&k).unwrap(), v));
    assert!(kv.is_empty());
}
