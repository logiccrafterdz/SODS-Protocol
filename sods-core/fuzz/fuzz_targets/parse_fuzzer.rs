#![no_main]
use libfuzzer_sys::fuzz_target;
use sods_core::pattern::BehavioralPattern;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // We wrap it in a catch_unwind to be extra safe, though parse doesn't use unsafe.
        let _ = std::panic::catch_unwind(|| {
            let _ = BehavioralPattern::parse(s);
        });
    }
});
