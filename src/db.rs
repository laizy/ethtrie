use ethereum_types::H256;
use hashbrown::HashMap ;

/// "DB" defines the "trait" of trie and database interaction.
/// You should first write the data to the cache and write the data
/// to the database in bulk after the end of a set of operations.
pub trait HashDB {
    fn get(&self, key: &H256) -> Option<Vec<u8>>;

    fn contains(&self, key: &H256) -> bool;

    /// Insert data into the cache.
    fn insert(&mut self, key: H256, value: Vec<u8>);

    /// Insert data into the cache.
    fn remove(&mut self, key: &H256);

    /// Insert a batch of data into the cache.
    fn insert_batch(&mut self, keys: Vec<H256>, values: Vec<Vec<u8>>) {
        for i in 0..keys.len() {
            let key = keys[i].clone();
            let value = values[i].clone();
            self.insert(key, value);
        }
    }

    /// Remove a batch of data into the cache.
    fn remove_batch(&mut self, keys: &[H256]) {
        for key in keys {
            self.remove(key);
        }
    }

    /// Flush data to the DB from the cache.
    fn flush(&mut self);

    #[cfg(test)]
    fn len(&self) -> usize;
    #[cfg(test)]
    fn is_empty(&self) -> bool;
}

#[derive(Default, Debug)]
pub struct MemoryDB {
    // If "light" is true, the data is deleted from the database at the time of submission.
    light: bool,
    storage: HashMap<H256, Vec<u8>>,
}

impl MemoryDB {
    pub fn new(light: bool) -> Self {
        MemoryDB { light, storage: HashMap::new(), }
    }
}

impl HashDB for MemoryDB {
    fn get(&self, key: &H256) -> Option<Vec<u8>> {
        self.storage.get(key).cloned()
    }

    fn contains(&self, key: &H256) -> bool {
        self.storage.contains_key(key)
    }

    fn insert(&mut self, key: H256, value: Vec<u8>) {
        self.storage.insert(key, value);
    }

    fn remove(&mut self, key: &H256) {
        if self.light {
            self.storage.remove(key);
        }
    }

    fn flush(&mut self) {}

    #[cfg(test)]
    fn len(&self) -> usize {
        self.storage.len()
    }
    #[cfg(test)]
    fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }
}
