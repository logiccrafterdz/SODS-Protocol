use serde::{Deserialize, Serialize};
use sods_core::symbol::BehavioralSymbol;

#[derive(Serialize, Deserialize, Debug)]
pub struct BehavioralProofInput {
    pub symbols: Vec<BehavioralSymbol>,
    pub pattern: String,
}
