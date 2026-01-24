#![no_main]
use libfuzzer_sys::fuzz_target;
use sods_core::SymbolDictionary;
use ethers_core::types::Log;

fuzz_target!(|data: &[u8]| {
    // Attempt to interpret bytes as a partial log or raw RLP
    let dictionary = SymbolDictionary::default();
    
    // Create a mock log with randomized topics and data
    let mut log = Log::default();
    if data.len() >= 32 {
        log.topics.push(ethers_core::types::H256::from_slice(&data[0..32]));
    }
    if data.len() >= 64 {
        log.data = ethers_core::types::Bytes::from(data[32..].to_vec());
    }
    
    // Fuzz the parser entrance
    let _ = dictionary.parse_log(&log);
});
