extern crate alloc;
use crate::trie::TrieResult;
use crate::{keccak256, HashDB, PatriciaTrie, PatriciaTrieMut, H256};
use alloc::vec::Vec;

pub struct TrieDB<'db, D: HashDB> {
    trie: PatriciaTrie<'db, D>,
}

pub struct TrieDBMut<'db, D: HashDB> {
    trie: PatriciaTrieMut<'db, D>,
}

pub struct SecTrieDB<'db, D: HashDB> {
    trie: TrieDB<'db, D>,
}

pub struct SecTrieDBMut<'db, D: HashDB> {
    trie: TrieDBMut<'db, D>,
}

impl<'db, D: HashDB> TrieDB<'db, D> {
    pub fn hashdb(&self) -> &D {
        self.trie.hashdb()
    }

    pub fn iter(&self) -> impl Iterator<Item = (H256, Vec<u8>)> + '_ {
        self.trie
            .iter()
            .map(|(key, value)| (H256::from_slice(&key), value))
    }

    pub fn new(db: &'db mut D) -> Self {
        Self {
            trie: PatriciaTrie::new(db),
        }
    }

    pub fn from(db: &'db D, root: H256) -> TrieResult<Self> {
        Ok(Self {
            trie: PatriciaTrie::from(db, root)?,
        })
    }
}

impl<'db, D: HashDB> SecTrieDB<'db, D> {
    pub fn new(db: &'db mut D) -> Self {
        Self {
            trie: TrieDB::new(db),
        }
    }

    pub fn from(db: &'db D, root: H256) -> TrieResult<Self> {
        Ok(Self {
            trie: TrieDB::from(db, root)?,
        })
    }

    pub fn hashdb(&self) -> &D {
        self.trie.hashdb()
    }
}

impl<'db, D: HashDB> SecTrieDBMut<'db, D> {
    pub fn new(db: &'db mut D) -> Self {
        Self {
            trie: TrieDBMut::new(db),
        }
    }

    pub fn from(db: &'db mut D, root: H256) -> TrieResult<Self> {
        Ok(Self {
            trie: TrieDBMut::from(db, root)?,
        })
    }

    pub fn hashdb_mut(&mut self) -> &mut D {
        self.trie.hashdb_mut()
    }

    pub fn hashdb(&self) -> &D {
        self.trie.hashdb()
    }
}

impl<'db, D: HashDB> TrieDBMut<'db, D> {
    pub fn iter(&self) -> impl Iterator<Item = (H256, Vec<u8>)> + '_ {
        self.trie
            .iter()
            .map(|(key, value)| (H256::from_slice(&key), value))
    }

    pub fn new(db: &'db mut D) -> Self {
        Self {
            trie: PatriciaTrieMut::new(db),
        }
    }

    pub fn from(db: &'db mut D, root: H256) -> TrieResult<Self> {
        Ok(Self {
            trie: PatriciaTrieMut::from(db, root)?,
        })
    }
    pub fn hashdb(&self) -> &D {
        self.trie.hashdb()
    }

    pub fn hashdb_mut(&mut self) -> &mut D {
        self.trie.hashdb_mut()
    }

    /// Inserts value into trie and modifies it if it exists
    pub fn insert(&mut self, key: &H256, value: Vec<u8>) -> TrieResult<()> {
        self.trie.insert(key.as_bytes(), value)
    }

    /// Removes any existing value for key from the trie.
    pub fn remove(&mut self, key: &H256) -> TrieResult<bool> {
        self.trie.remove(key.as_bytes())
    }

    /// Saves all the nodes in the db, clears the cache data, recalculates the root.
    /// Returns the root hash of the trie.
    pub fn root(&mut self) -> TrieResult<H256> {
        self.trie.root()
    }

    pub fn get(&self, key: &H256) -> TrieResult<Option<Vec<u8>>> {
        self.trie.get(key.as_bytes())
    }

    pub fn contains(&self, key: &H256) -> TrieResult<bool> {
        self.trie.contains(key.as_bytes())
    }

    pub fn get_proof(&self, key: &H256) -> TrieResult<Vec<Vec<u8>>> {
        self.trie.get_proof(key.as_bytes())
    }

    pub fn verify_proof(
        &self,
        root_hash: H256,
        key: &H256,
        proof: Vec<Vec<u8>>,
    ) -> TrieResult<Option<Vec<u8>>> {
        self.trie.verify_proof(root_hash, key.as_bytes(), proof)
    }
}

impl<'db, D: HashDB> TrieDB<'db, D> {
    pub fn get(&self, key: &H256) -> TrieResult<Option<Vec<u8>>> {
        self.trie.get(key.as_bytes())
    }

    pub fn contains(&self, key: &H256) -> TrieResult<bool> {
        self.trie.contains(key.as_bytes())
    }

    pub fn get_proof(&self, key: &H256) -> TrieResult<Vec<Vec<u8>>> {
        self.trie.get_proof(key.as_bytes())
    }

    pub fn verify_proof(
        &self,
        root_hash: H256,
        key: &H256,
        proof: Vec<Vec<u8>>,
    ) -> TrieResult<Option<Vec<u8>>> {
        self.trie.verify_proof(root_hash, key.as_bytes(), proof)
    }
}

impl<'db, D: HashDB> SecTrieDB<'db, D> {
    /// Returns the value for key stored in the trie.
    pub fn get(&self, key: &H256) -> TrieResult<Option<Vec<u8>>> {
        let key = keccak256(key.as_bytes());
        self.trie.get(&key)
    }

    /// Checks that the key is present in the trie
    pub fn contains(&self, key: &H256) -> TrieResult<bool> {
        self.trie.contains(&keccak256(key.as_bytes()))
    }

    pub fn trie(&self) -> &TrieDB<'db, D> {
        &self.trie
    }
}

impl<'db, D: HashDB> SecTrieDBMut<'db, D> {
    pub fn get(&self, key: &H256) -> TrieResult<Option<Vec<u8>>> {
        let key = keccak256(key.as_bytes());
        self.trie.get(&key)
    }

    pub fn contains(&self, key: &H256) -> TrieResult<bool> {
        self.trie.contains(&keccak256(key.as_bytes()))
    }

    pub fn insert(&mut self, key: &H256, value: Vec<u8>) -> TrieResult<()> {
        self.trie.insert(&keccak256(key.as_bytes()), value)
    }

    pub fn remove(&mut self, key: &H256) -> TrieResult<bool> {
        self.trie.remove(&keccak256(key.as_bytes()))
    }

    pub fn root(&mut self) -> TrieResult<H256> {
        self.trie.root()
    }

    pub fn trie(&self) -> &TrieDBMut<'db, D> {
        &self.trie
    }

    pub fn trie_mut(&mut self) -> &mut TrieDBMut<'db, D> {
        &mut self.trie
    }
}
