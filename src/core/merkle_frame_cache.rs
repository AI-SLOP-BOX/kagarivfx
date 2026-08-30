#![allow(dead_code)]
use std::collections::HashMap;

/// Merkle Tree Node representing a hashed layer node or compositing sub-tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MerkleHash(pub String);

/// Merkle Tree Content-Addressed Frame Cache Engine (Git-like zero-copy cache).
pub struct MerkleFrameCache {
    cache_store: HashMap<MerkleHash, Vec<u8>>,
}

impl Default for MerkleFrameCache {
    fn default() -> Self {
        Self::new()
    }
}

impl MerkleFrameCache {
    pub fn new() -> Self {
        Self {
            cache_store: HashMap::new(),
        }
    }

    /// Computes Merkle Hash for a layer node given its ID, parameters, frame, and parent Merkle Hash.
    pub fn compute_node_hash(
        layer_id: &str,
        params_signature: &str,
        frame: u32,
        parent_hash: Option<&MerkleHash>,
    ) -> MerkleHash {
        // Simple deterministic FNV 64-bit hash for Merkle key generation
        let mut hasher: u64 = 0xcbf29ce484222325;
        let mut feed = |bytes: &[u8]| {
            hasher ^= bytes.len() as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
            for byte in bytes {
                hasher ^= *byte as u64;
                hasher = hasher.wrapping_mul(0x100000001b3);
            }
        };
        feed(layer_id.as_bytes());
        feed(params_signature.as_bytes());
        feed(&frame.to_le_bytes());
        feed(parent_hash.map(|h| h.0.as_bytes()).unwrap_or(b"ROOT"));

        for byte in b"aura-merkle-v1" {
            hasher ^= *byte as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }

        MerkleHash(format!("{:016x}", hasher))
    }

    /// Checks if rendered frame buffer exists in Merkle Cache.
    pub fn get(&self, hash: &MerkleHash) -> Option<&Vec<u8>> {
        self.cache_store.get(hash)
    }

    /// Stores rendered frame buffer into Merkle Cache.
    pub fn insert(&mut self, hash: MerkleHash, buffer: Vec<u8>) {
        self.cache_store.insert(hash, buffer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_cache_hit() {
        let mut cache = MerkleFrameCache::new();
        let hash = MerkleFrameCache::compute_node_hash("layer_1", "pos(100,200)", 5, None);

        let pixel_data = vec![255u8; 16];
        cache.insert(hash.clone(), pixel_data.clone());

        let cached = cache.get(&hash);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 16);
    }

    #[test]
    fn test_merkle_hash_preserves_field_boundaries() {
        let first = MerkleFrameCache::compute_node_hash("a:b", "c", 1, None);
        let second = MerkleFrameCache::compute_node_hash("a", "b:c", 1, None);
        assert_ne!(first, second);
    }
}
