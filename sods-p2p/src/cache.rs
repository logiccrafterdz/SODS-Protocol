//! LRU cache for verified blocks and BMT roots.

use lru::LruCache;
use sods_core::BehavioralSymbol;
use std::num::NonZeroUsize;
use std::time::Instant;

/// Default cache capacity (number of blocks).
const DEFAULT_CAPACITY: usize = 1000;

/// A cached block with its BMT data.
#[derive(Debug, Clone)]
pub struct CachedBlock {
    /// Behavioral Merkle Root
    pub bmt_root: [u8; 32],
    /// Parsed symbols from the block
    pub symbols: Vec<BehavioralSymbol>,
    /// When this entry was cached
    pub cached_at: Instant,
}

impl CachedBlock {
    /// Create a new cached block entry.
    pub fn new(bmt_root: [u8; 32], symbols: Vec<BehavioralSymbol>) -> Self {
        Self {
            bmt_root,
            symbols,
            cached_at: Instant::now(),
        }
    }

    /// Check if a symbol exists in this cached block.
    pub fn has_symbol(&self, symbol: &str) -> bool {
        self.symbols.iter().any(|s| s.symbol() == symbol)
    }

    /// Count occurrences of a symbol.
    pub fn count_symbol(&self, symbol: &str) -> usize {
        self.symbols.iter().filter(|s| s.symbol() == symbol).count()
    }
}

/// LRU cache for block data.
pub struct BlockCache {
    cache: LruCache<u64, CachedBlock>,
}

impl BlockCache {
    /// Create a new block cache with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    /// Create a new block cache with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: LruCache::new(cap),
        }
    }

    /// Get a cached block by number.
    pub fn get(&mut self, block_number: u64) -> Option<&CachedBlock> {
        self.cache.get(&block_number)
    }

    /// Insert a block into the cache.
    pub fn insert(&mut self, block_number: u64, block: CachedBlock) {
        self.cache.put(block_number, block);
    }

    /// Check if a block is cached.
    pub fn contains(&self, block_number: u64) -> bool {
        self.cache.contains(&block_number)
    }

    /// Get the number of cached blocks.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for BlockCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_insert_get() {
        let mut cache = BlockCache::new();
        let block = CachedBlock::new([0xAB; 32], vec![]);
        
        cache.insert(12345, block);
        
        assert!(cache.contains(12345));
        assert!(cache.get(12345).is_some());
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = BlockCache::with_capacity(2);
        
        cache.insert(1, CachedBlock::new([1; 32], vec![]));
        cache.insert(2, CachedBlock::new([2; 32], vec![]));
        cache.insert(3, CachedBlock::new([3; 32], vec![]));
        
        // Block 1 should be evicted
        assert!(!cache.contains(1));
        assert!(cache.contains(2));
        assert!(cache.contains(3));
    }

    #[test]
    fn test_cached_block_has_symbol() {
        let symbols = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Dep", 1),
        ];
        let block = CachedBlock::new([0; 32], symbols);
        
        assert!(block.has_symbol("Tf"));
        assert!(block.has_symbol("Dep"));
        assert!(!block.has_symbol("Wdw"));
    }
}
