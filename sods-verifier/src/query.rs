//! Query parser for symbol validation.
//!
//! Validates that query symbols are in the supported registry.

use crate::error::{Result, SodsVerifierError};

/// Supported symbols in the SODS core registry.
const SUPPORTED_SYMBOLS: &[&str] = &["Tf", "Dep", "Wdw", "Sw", "LP+", "LP-"];

/// Query parser for validating symbol queries.
///
/// Currently supports only simple single-symbol queries.
/// Future versions will support pattern sequences (e.g., `LP+ → LP-`).
#[derive(Debug, Clone, Default)]
pub struct QueryParser;

impl QueryParser {
    /// Create a new query parser.
    pub fn new() -> Self {
        Self
    }

    /// Validate that a symbol is supported.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol to validate
    ///
    /// # Returns
    ///
    /// Ok(()) if valid, or `UnsupportedSymbol` error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use sods_verifier::QueryParser;
    ///
    /// let parser = QueryParser::new();
    /// assert!(parser.validate_symbol("Tf").is_ok());
    /// assert!(parser.validate_symbol("BadSymbol").is_err());
    /// ```
    pub fn validate_symbol(&self, symbol: &str) -> Result<()> {
        if SUPPORTED_SYMBOLS.contains(&symbol) {
            Ok(())
        } else {
            Err(SodsVerifierError::UnsupportedSymbol(symbol.to_string()))
        }
    }

    /// Check if a symbol is supported without returning an error.
    pub fn is_supported(&self, symbol: &str) -> bool {
        SUPPORTED_SYMBOLS.contains(&symbol)
    }

    /// Get list of all supported symbols.
    pub fn supported_symbols(&self) -> &'static [&'static str] {
        SUPPORTED_SYMBOLS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_symbols() {
        let parser = QueryParser::new();
        
        for symbol in SUPPORTED_SYMBOLS {
            assert!(parser.validate_symbol(symbol).is_ok());
            assert!(parser.is_supported(symbol));
        }
    }

    #[test]
    fn test_invalid_symbol() {
        let parser = QueryParser::new();
        
        assert!(parser.validate_symbol("Unknown").is_err());
        assert!(parser.validate_symbol("").is_err());
        assert!(parser.validate_symbol("tf").is_err()); // Case sensitive
        assert!(!parser.is_supported("BadSymbol"));
    }

    #[test]
    fn test_supported_symbols_list() {
        let parser = QueryParser::new();
        let symbols = parser.supported_symbols();
        
        assert_eq!(symbols.len(), 6);
        assert!(symbols.contains(&"Tf"));
        assert!(symbols.contains(&"LP+"));
    }
}
