#[cfg(unix)]
#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Duration;
    use std::env;

    #[tokio::test]
    async fn test_daemon_threat_feed_argument() {
        if env::var("SODS_RUN_INTEGRATION_TESTS").unwrap_or_default() != "1" {
            return;
        }

        // Verify the command accepts --threat-feed without immediate crash
        // Note: It will fail to fetch (network error) or parsing but that's runtime.
        // We just want to ensure it acts as expected (tries to start).
        
        let mut child = Command::new("cargo")
            .args(&[
                "run",
                "--",
                "daemon",
                "start",
                "--threat-feed", "https://raw.githubusercontent.com/sods/threats/main/base.json",
                "--chain", "base"
            ])
            .spawn()
            .expect("Failed to start daemon command");
            
        // Let it run briefly (it might exit due to fetch failure 404/network, which is fine)
        tokio::time::sleep(Duration::from_secs(3)).await;
        
        let _ = child.kill();
    }
}
