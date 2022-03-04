use std::error::Error;
use std::fmt;

use rlp::DecoderError;

#[derive(Debug)]
pub enum TrieError {
    Decoder(DecoderError),
    InvalidData,
    InvalidStateRoot,
    InvalidProof,
}

impl Error for TrieError {}

impl fmt::Display for TrieError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match *self {
            TrieError::Decoder(ref err) => format!("trie error: {:?}", err),
            TrieError::InvalidData => "trie error: invalid data".to_owned(),
            TrieError::InvalidStateRoot => "trie error: invalid state root".to_owned(),
            TrieError::InvalidProof => "trie error: invalid proof".to_owned(),
        };
        write!(f, "{}", printable)
    }
}

impl From<DecoderError> for TrieError {
    fn from(error: DecoderError) -> Self {
        TrieError::Decoder(error)
    }
}
