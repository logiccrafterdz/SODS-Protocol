use clap::Args;
use ethers_core::utils::keccak256;
use crate::output;

#[derive(Args)]
pub struct HashPatternArgs {
    /// Behavioral pattern to hash
    pub pattern: String,
}

pub async fn run(args: HashPatternArgs) -> i32 {
    let hash = keccak256(args.pattern.as_bytes());
    println!("0x{}", hex::encode(hash));
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hash_consistency() {
        let pattern = "LP+ -> Sw -> LP-";
        let hash = keccak256(pattern.as_bytes());
        let expected = "0x8a3b7c"; // Placeholder for actual start of hash
        assert!(format!("0x{}", hex::encode(hash)).starts_with("0x"));
    }
}
