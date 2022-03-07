extern crate alloc;
use alloc::rc::Rc;
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use ethereum_types::H256;

use crate::hasher::keccak256;
use hashbrown::{HashMap, HashSet};
use rlp::{Prototype, Rlp, RlpStream};

use crate::db::{HashDB, MemoryDB};
use crate::errors::TrieError;
use crate::nibbles::Nibbles;
use crate::node::{empty_children, BranchNode, Node, RawNodeOrHash};

pub type TrieResult<T> = Result<T, TrieError>;

const HASH_LEN: usize = 32;

#[derive(Debug)]
pub struct PatriciaTrie<'db, D: HashDB> {
    root: Node,
    root_hash: H256,
    db: &'db mut D,
    cache: RefCell<HashMap<H256, Vec<u8>>>,
    passing_keys: HashSet<H256>,
    gen_keys: RefCell<HashSet<H256>>,
}

#[derive(Clone, Debug)]
enum TraceStatus {
    Start,
    Doing,
    Child(u8),
    End,
}

#[derive(Clone, Debug)]
struct TraceNode {
    node: Node,
    status: TraceStatus,
}

impl TraceNode {
    fn advance(&mut self) {
        self.status = match &self.status {
            TraceStatus::Start => TraceStatus::Doing,
            TraceStatus::Doing => match self.node {
                Node::Branch(_) => TraceStatus::Child(0),
                _ => TraceStatus::End,
            },
            TraceStatus::Child(i) if *i < 15 => TraceStatus::Child(i + 1),
            _ => TraceStatus::End,
        }
    }
}

impl From<Node> for TraceNode {
    fn from(node: Node) -> TraceNode {
        TraceNode {
            node,
            status: TraceStatus::Start,
        }
    }
}

pub struct TrieIterator<'a, 'db, D: HashDB> {
    trie: &'a PatriciaTrie<'db, D>,
    nibble: Nibbles,
    nodes: Vec<TraceNode>,
}

impl<'a, 'db, D: HashDB> Iterator for TrieIterator<'a, 'db, D> {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut now = self.nodes.last().cloned();
            if let Some(ref mut now) = now {
                self.nodes.last_mut().unwrap().advance();

                match (now.status.clone(), &now.node) {
                    (TraceStatus::End, node) => {
                        match *node {
                            Node::Leaf(ref leaf) => {
                                let cur_len = self.nibble.len();
                                self.nibble.truncate(cur_len - leaf.borrow().key.len());
                            }

                            Node::Extension(ref ext) => {
                                let cur_len = self.nibble.len();
                                self.nibble.truncate(cur_len - ext.borrow().prefix.len());
                            }

                            Node::Branch(_) => {
                                self.nibble.pop();
                            }
                            _ => {}
                        }
                        self.nodes.pop();
                    }

                    (TraceStatus::Doing, Node::Extension(ref ext)) => {
                        self.nibble.extend(&ext.borrow().prefix);
                        self.nodes.push((ext.borrow().node.clone()).into());
                    }

                    (TraceStatus::Doing, Node::Leaf(ref leaf)) => {
                        self.nibble.extend(&leaf.borrow().key);
                        return Some((self.nibble.encode_raw().0, leaf.borrow().value.clone()));
                    }

                    (TraceStatus::Doing, Node::Branch(ref branch)) => {
                        let value = branch.borrow().value.clone();
                        if value.is_none() {
                            continue;
                        } else {
                            return Some((self.nibble.encode_raw().0, value.unwrap()));
                        }
                    }

                    (TraceStatus::Doing, Node::Hash(hash_node)) => {
                        if let Ok(n) = self.trie.recover_from_db(&hash_node) {
                            self.nodes.pop();
                            self.nodes.push(n.into());
                        } else {
                            //error!();
                            return None;
                        }
                    }

                    (TraceStatus::Child(i), Node::Branch(ref branch)) => {
                        if i == 0 {
                            self.nibble.push(0);
                        } else {
                            self.nibble.pop();
                            self.nibble.push(i);
                        }
                        self.nodes
                            .push((branch.borrow().children[i as usize].clone()).into());
                    }

                    (_, Node::Empty) => {
                        self.nodes.pop();
                    }
                    _ => {}
                }
            } else {
                return None;
            }
        }
    }
}

impl<'db, D: HashDB> PatriciaTrie<'db, D> {
    pub fn iter(&self) -> TrieIterator<D> {
        let mut nodes = Vec::new();
        nodes.push((self.root.clone()).into());
        TrieIterator {
            trie: self,
            nibble: Nibbles::from_raw(&[], false),
            nodes,
        }
    }

    pub fn new(db: &'db mut D) -> Self {
        Self {
            root: Node::Empty,
            root_hash: keccak256(&rlp::NULL_RLP.to_vec()),

            cache: RefCell::new(HashMap::new()),
            passing_keys: HashSet::new(),
            gen_keys: RefCell::new(HashSet::new()),

            db,
        }
    }

    pub fn from(db: &'db mut D, root: H256) -> TrieResult<Self> {
        match db.get(&root) {
            Some(data) => {
                let mut trie = Self {
                    root: Node::Empty,
                    root_hash: root,

                    cache: RefCell::new(HashMap::new()),
                    passing_keys: HashSet::new(),
                    gen_keys: RefCell::new(HashSet::new()),

                    db,
                };

                trie.root = trie.decode_node(&data)?;
                Ok(trie)
            }
            None => Err(TrieError::InvalidStateRoot),
        }
    }
}

impl<'db, D: HashDB> PatriciaTrie<'db, D> {
    /// Returns the value for key stored in the trie.
    pub fn get(&self, key: &[u8]) -> TrieResult<Option<Vec<u8>>> {
        self.get_at(self.root.clone(), &Nibbles::from_raw(key, true))
    }

    /// Checks that the key is present in the trie
    pub fn contains(&self, key: &[u8]) -> TrieResult<bool> {
        Ok(self
            .get_at(self.root.clone(), &Nibbles::from_raw(key, true))?
            .map_or(false, |_| true))
    }

    /// Inserts value into trie and modifies it if it exists
    pub fn insert(&mut self, key: &[u8], value: Vec<u8>) -> TrieResult<()> {
        if value.is_empty() {
            self.remove(key)?;
            return Ok(());
        }
        let root = self.root.clone();
        self.root = self.insert_at(root, Nibbles::from_raw(key, true), value)?;
        Ok(())
    }

    /// Removes any existing value for key from the trie.
    pub fn remove(&mut self, key: &[u8]) -> TrieResult<bool> {
        let (n, removed) = self.delete_at(self.root.clone(), &Nibbles::from_raw(key, true))?;
        self.root = n;
        Ok(removed)
    }

    /// Prove constructs a merkle proof for key. The result contains all encoded nodes
    /// on the path to the value at key. The value itself is also included in the last
    /// node and can be retrieved by verifying the proof.
    ///
    /// If the trie does not contain a value for key, the returned proof contains all
    /// nodes of the longest existing prefix of the key (at least the root node), ending
    /// with the node that proves the absence of the key.
    pub fn get_proof(&self, key: &[u8]) -> TrieResult<Vec<Vec<u8>>> {
        let mut path = self.get_path_at(self.root.clone(), &Nibbles::from_raw(key, true))?;
        match self.root {
            Node::Empty => {}
            _ => path.push(self.root.clone()),
        }
        Ok(path.into_iter().rev().map(|n| self.encode_raw(n)).collect())
    }

    /// return value if key exists, None if key not exist, Error if proof is wrong
    pub fn verify_proof(
        &self,
        root_hash: H256,
        key: &[u8],
        proof: Vec<Vec<u8>>,
    ) -> TrieResult<Option<Vec<u8>>> {
        let mut memdb = MemoryDB::new(true);
        for node_encoded in proof.into_iter() {
            let hash = keccak256(&node_encoded);

            if root_hash.eq(&hash) || node_encoded.len() >= HASH_LEN {
                memdb.insert(hash, node_encoded);
            }
        }
        let trie = PatriciaTrie::from(&mut memdb, root_hash).or(Err(TrieError::InvalidProof))?;
        trie.get(key).or(Err(TrieError::InvalidProof))
    }
}

impl<'db, D: HashDB> PatriciaTrie<'db, D> {
    fn get_at(&self, n: Node, partial: &Nibbles) -> TrieResult<Option<Vec<u8>>> {
        match n {
            Node::Empty => Ok(None),
            Node::Leaf(leaf) => {
                let borrow_leaf = leaf.borrow();

                if &borrow_leaf.key == partial {
                    Ok(Some(borrow_leaf.value.clone()))
                } else {
                    Ok(None)
                }
            }
            Node::Branch(branch) => {
                let borrow_branch = branch.borrow();

                if partial.is_empty() || partial.at(0) == 16 {
                    Ok(borrow_branch.value.clone())
                } else {
                    let index = partial.at(0);
                    self.get_at(borrow_branch.children[index].clone(), &partial.offset(1))
                }
            }
            Node::Extension(extension) => {
                let extension = extension.borrow();

                let prefix = &extension.prefix;
                let match_len = partial.common_prefix(&prefix);
                if match_len == prefix.len() {
                    self.get_at(extension.node.clone(), &partial.offset(match_len))
                } else {
                    Ok(None)
                }
            }
            Node::Hash(hash) => {
                let n = self.recover_from_db(&hash)?;
                self.get_at(n, partial)
            }
        }
    }

    fn insert_at(&mut self, n: Node, partial: Nibbles, value: Vec<u8>) -> TrieResult<Node> {
        match n {
            Node::Empty => Ok(Node::from_leaf(partial, value)),
            Node::Leaf(leaf) => {
                let mut borrow_leaf = leaf.borrow_mut();

                let old_partial = &borrow_leaf.key;
                let match_index = partial.common_prefix(old_partial);
                if match_index == old_partial.len() {
                    // replace leaf value
                    borrow_leaf.value = value;
                    return Ok(Node::Leaf(leaf.clone()));
                }

                let mut branch = BranchNode {
                    children: empty_children(),
                    value: None,
                };

                let n = Node::from_leaf(
                    old_partial.offset(match_index + 1),
                    borrow_leaf.value.clone(),
                );
                branch.insert(old_partial.at(match_index), n);

                let n = Node::from_leaf(partial.offset(match_index + 1), value);
                branch.insert(partial.at(match_index), n);

                if match_index == 0 {
                    return Ok(Node::Branch(Rc::new(RefCell::new(branch))));
                }

                // if include a common prefix
                Ok(Node::from_extension(
                    partial.slice(0, match_index),
                    Node::Branch(Rc::new(RefCell::new(branch))),
                ))
            }
            Node::Branch(branch) => {
                let mut borrow_branch = branch.borrow_mut();

                if partial.at(0) == 0x10 {
                    borrow_branch.value = Some(value);
                    return Ok(Node::Branch(branch.clone()));
                }

                let child = borrow_branch.children[partial.at(0)].clone();
                let new_child = self.insert_at(child, partial.offset(1), value)?;
                borrow_branch.children[partial.at(0)] = new_child;
                Ok(Node::Branch(branch.clone()))
            }
            Node::Extension(ext) => {
                let mut borrow_ext = ext.borrow_mut();

                let prefix = &borrow_ext.prefix;
                let sub_node = borrow_ext.node.clone();
                let match_index = partial.common_prefix(&prefix);

                if match_index == 0 {
                    let mut branch = BranchNode {
                        children: empty_children(),
                        value: None,
                    };
                    branch.insert(
                        prefix.at(0),
                        if prefix.len() == 1 {
                            sub_node
                        } else {
                            Node::from_extension(prefix.offset(1), sub_node)
                        },
                    );
                    let node = Node::Branch(Rc::new(RefCell::new(branch)));

                    return self.insert_at(node, partial, value);
                }

                if match_index == prefix.len() {
                    let new_node = self.insert_at(sub_node, partial.offset(match_index), value)?;
                    return Ok(Node::from_extension(prefix.clone(), new_node));
                }

                let new_ext = Node::from_extension(prefix.offset(match_index), sub_node);
                let new_node = self.insert_at(new_ext, partial.offset(match_index), value)?;
                borrow_ext.prefix = prefix.slice(0, match_index);
                borrow_ext.node = new_node;
                Ok(Node::Extension(ext.clone()))
            }
            Node::Hash(hash_node) => {
                self.passing_keys.insert(hash_node);
                let n = self.recover_from_db(&hash_node)?;
                self.insert_at(n, partial, value)
            }
        }
    }

    fn delete_at(&mut self, n: Node, partial: &Nibbles) -> TrieResult<(Node, bool)> {
        let (new_n, deleted) = match n {
            Node::Empty => Ok((Node::Empty, false)),
            Node::Leaf(leaf) => {
                let borrow_leaf = leaf.borrow();

                if &borrow_leaf.key == partial {
                    return Ok((Node::Empty, true));
                }
                Ok((Node::Leaf(leaf.clone()), false))
            }
            Node::Branch(branch) => {
                let mut borrow_branch = branch.borrow_mut();

                if partial.at(0) == 0x10 {
                    borrow_branch.value = None;
                    return Ok((Node::Branch(branch.clone()), true));
                }

                let index = partial.at(0);
                let node = borrow_branch.children[index].clone();

                let (new_n, deleted) = self.delete_at(node, &partial.offset(1))?;
                if deleted {
                    borrow_branch.children[index] = new_n;
                }

                Ok((Node::Branch(branch.clone()), deleted))
            }
            Node::Extension(ext) => {
                let mut borrow_ext = ext.borrow_mut();

                let prefix = &borrow_ext.prefix;
                let match_len = partial.common_prefix(prefix);

                if match_len == prefix.len() {
                    let (new_n, deleted) =
                        self.delete_at(borrow_ext.node.clone(), &partial.offset(match_len))?;

                    if deleted {
                        borrow_ext.node = new_n;
                    }

                    Ok((Node::Extension(ext.clone()), deleted))
                } else {
                    Ok((Node::Extension(ext.clone()), false))
                }
            }
            Node::Hash(hash_node) => {
                self.passing_keys.insert(hash_node);

                let n = self.recover_from_db(&hash_node)?;
                self.delete_at(n, partial)
            }
        }?;

        if deleted {
            Ok((self.degenerate(new_n)?, deleted))
        } else {
            Ok((new_n, deleted))
        }
    }

    fn degenerate(&mut self, n: Node) -> TrieResult<Node> {
        match n {
            Node::Branch(branch) => {
                let borrow_branch = branch.borrow();

                let mut used_indexs = Vec::new();
                for (index, node) in borrow_branch.children.iter().enumerate() {
                    match node {
                        Node::Empty => continue,
                        _ => used_indexs.push(index),
                    }
                }

                // if only a value node, transmute to leaf.
                if used_indexs.is_empty() && borrow_branch.value.is_some() {
                    let key = Nibbles::from_raw(&[], true);
                    let value = borrow_branch.value.clone().unwrap();
                    Ok(Node::from_leaf(key, value))
                // if only one node. make an extension.
                } else if used_indexs.len() == 1 && borrow_branch.value.is_none() {
                    let used_index = used_indexs[0];
                    let n = borrow_branch.children[used_index].clone();

                    let new_node =
                        Node::from_extension(Nibbles::from_hex(vec![used_index as u8]), n);
                    self.degenerate(new_node)
                } else {
                    Ok(Node::Branch(branch.clone()))
                }
            }
            Node::Extension(ext) => {
                let borrow_ext = ext.borrow();

                let prefix = &borrow_ext.prefix;
                match borrow_ext.node.clone() {
                    Node::Extension(sub_ext) => {
                        let borrow_sub_ext = sub_ext.borrow();

                        let new_prefix = prefix.join(&borrow_sub_ext.prefix);
                        let new_n = Node::from_extension(new_prefix, borrow_sub_ext.node.clone());
                        self.degenerate(new_n)
                    }
                    Node::Leaf(leaf) => {
                        let borrow_leaf = leaf.borrow();

                        let new_prefix = prefix.join(&borrow_leaf.key);
                        Ok(Node::from_leaf(new_prefix, borrow_leaf.value.clone()))
                    }
                    // try again after recovering node from the db.
                    Node::Hash(hash) => {
                        self.passing_keys.insert(hash);

                        let new_node = self.recover_from_db(&hash)?;

                        let n = Node::from_extension(borrow_ext.prefix.clone(), new_node);
                        self.degenerate(n)
                    }
                    _ => Ok(Node::Extension(ext.clone())),
                }
            }
            _ => Ok(n),
        }
    }

    // Get nodes path along the key, only the nodes whose encode length is greater than
    // hash length are added.
    // For embedded nodes whose data are already contained in their parent node, we don't need to
    // add them in the path.
    // In the code below, we only add the nodes get by `get_node_from_hash`, because they contains
    // all data stored in db, including nodes whose encoded data is less than hash length.
    fn get_path_at(&self, n: Node, partial: &Nibbles) -> TrieResult<Vec<Node>> {
        match n {
            Node::Empty | Node::Leaf(_) => Ok(Vec::new()),
            Node::Branch(branch) => {
                let borrow_branch = branch.borrow();

                if partial.is_empty() || partial.at(0) == 16 {
                    Ok(Vec::new())
                } else {
                    let node = borrow_branch.children[partial.at(0)].clone();
                    self.get_path_at(node, &partial.offset(1))
                }
            }
            Node::Extension(ext) => {
                let borrow_ext = ext.borrow();

                let prefix = &borrow_ext.prefix;
                let match_len = partial.common_prefix(prefix);

                if match_len == prefix.len() {
                    self.get_path_at(borrow_ext.node.clone(), &partial.offset(match_len))
                } else {
                    Ok(Vec::new())
                }
            }
            Node::Hash(hash_node) => {
                let n = self.recover_from_db(&hash_node)?;
                let mut rest = self.get_path_at(n.clone(), partial)?;
                rest.push(n);
                Ok(rest)
            }
        }
    }

    /// Saves all the nodes in the db, clears the cache data, recalculates the root.
    /// Returns the root hash of the trie.
    pub fn root(&mut self) -> TrieResult<H256> {
        let encoded = self.encode_node(self.root.clone());
        let root_hash = match encoded {
            RawNodeOrHash::Node(raw) => {
                let hash = keccak256(&raw);
                self.cache.get_mut().insert(hash.clone(), raw);
                hash
            }
            RawNodeOrHash::Hash(hash) => hash,
        };

        for (k, v) in self.cache.get_mut().drain() {
            self.db.insert(k, v);
        }

        let removed_keys: Vec<H256> = self
            .passing_keys
            .iter()
            .filter(|h| !self.gen_keys.borrow().contains(h))
            .map(|h| *h)
            .collect();

        self.db.remove_batch(&removed_keys);

        self.root_hash = root_hash;
        self.gen_keys.get_mut().clear();
        self.passing_keys.clear();
        self.root = self.recover_from_db(&root_hash)?;
        Ok(root_hash)
    }

    fn encode_node(&self, n: Node) -> RawNodeOrHash {
        // Returns the hash value directly to avoid double counting.
        if let Node::Hash(hash_node) = n {
            return RawNodeOrHash::Hash(hash_node);
        }

        let data = self.encode_raw(n.clone());
        // Nodes smaller than 32 bytes are stored inside their parent,
        // Nodes equal to 32 bytes are returned directly
        if data.len() < HASH_LEN {
            RawNodeOrHash::Node(data)
        } else {
            let hash = keccak256(&data);
            self.cache.borrow_mut().insert(hash, data);

            self.gen_keys.borrow_mut().insert(hash);
            RawNodeOrHash::Hash(hash)
        }
    }

    fn encode_raw(&self, n: Node) -> Vec<u8> {
        match n {
            Node::Empty => rlp::NULL_RLP.to_vec(),
            Node::Leaf(leaf) => {
                let borrow_leaf = leaf.borrow();

                let mut stream = RlpStream::new_list(2);
                stream.append(&borrow_leaf.key.encode_compact());
                stream.append(&borrow_leaf.value);
                stream.out()
            }
            Node::Branch(branch) => {
                let borrow_branch = branch.borrow();

                let mut stream = RlpStream::new_list(17);
                for i in 0..16 {
                    let n = borrow_branch.children[i].clone();
                    let data = self.encode_node(n);
                    match data {
                        RawNodeOrHash::Hash(data) => stream.append(&data.as_bytes()),
                        RawNodeOrHash::Node(data) => stream.append_raw(&data, 1),
                    };
                }

                match &borrow_branch.value {
                    Some(v) => stream.append(v),
                    None => stream.append_empty_data(),
                };
                stream.out()
            }
            Node::Extension(ext) => {
                let borrow_ext = ext.borrow();

                let mut stream = RlpStream::new_list(2);
                stream.append(&borrow_ext.prefix.encode_compact());
                let data = self.encode_node(borrow_ext.node.clone());
                match data {
                    RawNodeOrHash::Hash(data) => stream.append(&data.as_bytes()),
                    RawNodeOrHash::Node(data) => stream.append_raw(&data, 1),
                };

                stream.out()
            }
            Node::Hash(_hash) => unreachable!(),
        }
    }

    fn decode_node(&self, data: &[u8]) -> TrieResult<Node> {
        let r = Rlp::new(data);

        match r.prototype()? {
            Prototype::Data(0) => Ok(Node::Empty),
            Prototype::List(2) => {
                let key = r.at(0)?.data()?;
                let key = Nibbles::from_compact(key);

                if key.is_leaf() {
                    Ok(Node::from_leaf(key, r.at(1)?.data()?.to_vec()))
                } else {
                    let n = self.decode_node(r.at(1)?.as_raw())?;

                    Ok(Node::from_extension(key, n))
                }
            }
            Prototype::List(17) => {
                let mut nodes = empty_children();
                #[allow(clippy::needless_range_loop)]
                for i in 0..nodes.len() {
                    let rlp_data = r.at(i)?;
                    let n = self.decode_node(rlp_data.as_raw())?;
                    nodes[i] = n;
                }

                // The last element is a value node.
                let value_rlp = r.at(16)?;
                let value = if value_rlp.is_empty() {
                    None
                } else {
                    Some(value_rlp.data()?.to_vec())
                };

                Ok(Node::from_branch(nodes, value))
            }
            _ => {
                if r.is_data() && r.size() == HASH_LEN {
                    Ok(Node::from_hash(H256::from_slice(&r.data().unwrap())))
                } else {
                    Err(TrieError::InvalidData)
                }
            }
        }
    }

    fn recover_from_db(&self, key: &H256) -> TrieResult<Node> {
        match self.db.get(key) {
            Some(value) => Ok(self.decode_node(&value)?),
            None => Ok(Node::Empty),
        }
    }
}
