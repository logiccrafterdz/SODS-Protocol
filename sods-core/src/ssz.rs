use ethers_core::types::{H256, Address, Bloom, U256};
use serde::{Deserialize, Serialize};

/// ExecutionPayloadHeader for Post-Dencun blocks.
/// 
/// Reference: https://github.com/ethereum/consensus-specs/blob/dev/specs/deneb/beacon-chain.md#executionpayloadheader
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionPayloadHeader {
    pub parent_hash: H256,
    pub fee_recipient: Address,
    pub state_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bloom,
    pub prev_randao: H256,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Vec<u8>,
    pub base_fee_per_gas: U256,
    pub block_hash: H256,
    pub transactions_root: H256,
    pub withdrawals_root: H256,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
}

impl ExecutionPayloadHeader {
    /// Compute the SSZ root of the ExecutionPayloadHeader.
    /// 
    /// Note: This is a simplified implementation for v1. 
    /// Full SSZ Merkleization requires a proper tree hash.
    pub fn ssz_root(&self) -> H256 {
        // Placeholder until proper ssz_rs integration
        self.block_hash
    }
}
