//! Terminal output formatting.

use colored::Colorize;

/// Print a success message.
pub fn success(msg: &str) {
    println!("{} {}", "✓".green().bold(), msg.green());
}

/// Print an error message.
pub fn error(msg: &str) {
    eprintln!("{} {}", "✗".red().bold(), msg.red());
}

/// Print an info message.
pub fn info(msg: &str) {
    println!("{} {}", "→".cyan(), msg);
}

/// Print a warning message.
#[allow(dead_code)]
pub fn warn(msg: &str) {
    println!("{} {}", "!".yellow().bold(), msg.yellow());
}

/// Print a header.
pub fn header(msg: &str) {
    println!("\n{}", msg.white().bold());
    println!("{}", "─".repeat(msg.len()).dimmed());
}

/// Print a key-value pair.
pub fn kv(key: &str, value: &str) {
    println!("  {} {}", format!("{}:", key).dimmed(), value);
}

/// Print verification status.
pub fn verification_result(
    verified: bool,
    method: &str,
    proof_size: usize,
    time_ms: u64,
    occurrences: usize,
) {
    println!();
    if verified {
        success(&format!("Verified via {} ({} occurrences found)", method, occurrences));
        kv("Proof size", &format!("{} bytes", proof_size));
        kv("Total time", &format!("{:.2}s", time_ms as f64 / 1000.0));
        kv("Confidence", "High");
    } else {
        error("Verification failed");
    }
    println!();
}

/// Print a helpful hint.
pub fn hint(msg: &str) {
    println!("{} {}", "💡".dimmed(), msg.dimmed());
}
