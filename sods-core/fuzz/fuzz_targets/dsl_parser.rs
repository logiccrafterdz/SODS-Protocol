#![no_main]
use libfuzzer_sys::fuzz_target;
use sods_core::pattern::BehavioralPattern;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Attempt parsing
        let _ = BehavioralPattern::parse(s);
        
        // Also test with added length
        let mut long_s = s.to_string();
        long_s.push_str(" -> Sw");
        if long_s.len() <= 500 {
            let _ = BehavioralPattern::parse(&long_s);
        }
    }
});
