use criterion::{black_box, criterion_group, criterion_main, Criterion};
use sods_core::pattern::BehavioralPattern;

fn bench_simple_pattern(c: &mut Criterion) {
    c.bench_function("parse_simple", |b| {
        b.iter(|| BehavioralPattern::parse(black_box("Tf")))
    });
}

fn bench_complex_pattern(c: &mut Criterion) {
    let pattern = "Tf -> Sw{3,5} -> Tf where value > 100 ether";
    c.bench_function("parse_complex", |b| {
        b.iter(|| BehavioralPattern::parse(black_box(pattern)))
    });
}

fn bench_malicious_pattern_rejected(c: &mut Criterion) {
    // ReDoS attempt: very long pattern with excessive quantifiers (now rejected)
    let pattern = "Tf{1000}{1000}{1000}";
    c.bench_function("parse_malicious_rejected", |b| {
        b.iter(|| BehavioralPattern::parse(black_box(pattern)))
    });
}

fn bench_long_pattern_rejected(c: &mut Criterion) {
    let mut long_pattern = String::new();
    for _ in 0..100 {
        long_pattern.push_str("LongSymbolName -> ");
    }
    long_pattern.push_str("Tf");
    
    c.bench_function("parse_too_long_rejected", |b| {
        b.iter(|| BehavioralPattern::parse(black_box(&long_pattern)))
    });
}

criterion_group!(
    benches,
    bench_simple_pattern,
    bench_complex_pattern,
    bench_malicious_pattern_rejected,
    bench_long_pattern_rejected
);
criterion_main!(benches);
