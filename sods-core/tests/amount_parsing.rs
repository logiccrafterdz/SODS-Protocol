use sods_core::pattern::parse_amount;
use ethers_core::types::U256;

#[test]
fn test_large_ether_values() {
    assert_eq!(
        parse_amount("1000 ether").unwrap(),
        U256::from(1000u64) * U256::from(10u64).pow(U256::from(18))
    );
}

#[test]
fn test_decimal_precision() {
    let expected = U256::from(1_500_000_000_000_000_000u128); // 1.5 ether
    assert_eq!(parse_amount("1.5 ether").unwrap(), expected);
}

#[test]
fn test_max_ether_value() {
    // Should handle very large values without truncation
    let result = parse_amount("999999999999999999999 ether");
    assert!(result.is_ok());
}

#[test]
fn test_gwei_precision() {
    assert_eq!(
        parse_amount("1.5 gwei").unwrap(),
        U256::from(1_500_000_000u64)
    );
}

#[test]
fn test_invalid_amounts() {
    assert!(parse_amount("1.2.3 ether").is_err());
    assert!(parse_amount("abc ether").is_err());
    assert!(parse_amount("1e10 ether").is_err()); // No scientific notation
}

#[test]
fn test_wei_no_decimals() {
    assert_eq!(parse_amount("100 wei").unwrap(), U256::from(100u64));
    assert!(parse_amount("1.5 wei").is_err());
}

#[test]
fn test_high_decimal_precision() {
    // 0.000000000000000001 ether = 1 wei
    assert_eq!(parse_amount("0.000000000000000001 ether").unwrap(), U256::from(1u64));
    // More than 18 decimals should be truncated (floor)
    assert_eq!(parse_amount("0.0000000000000000019 ether").unwrap(), U256::from(1u64));
}
