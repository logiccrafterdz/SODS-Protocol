use std::process::Command;
use std::env;
use std::time::Duration;
use std::thread;

#[test]
fn test_autonomous_monitoring() {
    // Skip in CI if explicitly disabled
    if env::var("CI").is_ok() {
         if env::var("SODS_RUN_INTEGRATION_TESTS").unwrap_or_default() != "1" {
            println!("Skipping integration test in CI (set SODS_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
         }
    }

    let chain = "sepolia";
    let pattern = "Tf"; 
    
    // We start the monitor command with a timeout to ensure it exits
    // On Windows, `timeout` command is different or unavailable in the same way, 
    // so we'll actually rely on the fact that we can't easily kill it from strict `std::process::Command` without extra crates.
    // Instead, we'll try to run it with a very short timeout if possible or just check if it fails immediately?
    // Actually, "Unit test" scope said: "Simulate ... verify only blocks scanned".
    // Integration test scope said: "Run monitor ... Use timeout command or kill process".
    // Since we don't have `timeout` utility reliably on Windows CI environment, 
    // We will assume that if we can run it and it prints "Autonomous Monitor Active", it's working partially.
    // But `Command` waits for completion. We need to spawn a child.
    
    println!("Testing autonomous monitoring on chain: {}", chain);

    let mut child = Command::new("cargo")
        .args(&[
            "run",
            "--quiet",
            "--",
            "monitor",
            "--pattern",
            pattern,
            "--chain",
            chain,
            "--interval",
            "10s" // Minimum
        ])
        .spawn()
        .expect("Failed to start monitor command");

    // Let it run for 15 seconds (enough for initialization + maybe one poll?)
    thread::sleep(Duration::from_secs(15));

    // Kill it
    let _ = child.kill();
    
    // Check if it exited or we killed it. 
    // If it crashed early, wait() would return success/failure.
    // If we killed it, it means it was running successfully!
    let _status = child.wait().expect("Failed to wait on child");
    
    // If we killed it, standard exit code on Windows for terminated usually isn't 0.
    // But if it crashed *before* 5 seconds, it would have exited already.
    // We want to verify it survived 15s.
    // Actually, `wait()` will tell us the exit code. If we killed it, it's irrelevant. 
    // The strict assertion is: Did it crash *before* we killed it?
    // We can't know for sure here easily without `try_wait` (which `std` has).
    
    // Let's refine:
    // 1. Spawn
    // 2. Sleep 5s
    // 3. check `try_wait()`. If `Some`, it crashed/exited early -> FAIL.
    // 4. Kill.
    
    // Re-spawn for clean check
     let mut child_clean = Command::new("cargo")
        .args(&[
            "run",
            "--quiet",
            "--",
            "monitor",
            "--pattern",
            "Tf",
            "--chain",
            "sepolia",
            "--interval",
            "10s"
        ])
        .spawn()
        .expect("Failed to start monitor command");

    thread::sleep(Duration::from_secs(10));

    match child_clean.try_wait() {
        Ok(Some(status)) => {
            panic!("Monitor command exited prematurely with status: {}", status);
        },
        Ok(None) => {
            println!("Monitor command is still running after 10s. Success.");
            let _ = child_clean.kill();
        },
        Err(e) => panic!("Error attempting to wait: {}", e),
    }
}
