use ethereum_types::H256;
use tiny_keccak::Hasher;
use tiny_keccak::Keccak;

pub fn keccak256(data: &[u8]) -> H256 {
    let mut hasher = Keccak::v256();
    let mut result = H256::zero();
    hasher.update(data);
    hasher.finalize(result.as_mut());
    result
}
