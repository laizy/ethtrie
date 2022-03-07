use core::fmt;
use rlp::DecoderError;

#[derive(Debug)]
pub enum TrieError {
    Decoder(DecoderError),
    InvalidData,
    InvalidStateRoot,
    InvalidProof,
}

impl fmt::Display for TrieError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TrieError::Decoder(ref err) => write!(f, "trie error: {:?}", err),
            TrieError::InvalidData => write!(f, "trie error: invalid data"),
            TrieError::InvalidStateRoot => write!(f, "trie error: invalid state root"),
            TrieError::InvalidProof => write!(f, "trie error: invalid proof"),
        }
    }
}

impl From<DecoderError> for TrieError {
    fn from(error: DecoderError) -> Self {
        TrieError::Decoder(error)
    }
}
