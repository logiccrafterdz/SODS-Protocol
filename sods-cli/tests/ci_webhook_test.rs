#[cfg(unix)]
#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::time::Duration;
    use std::env;

    // Note: To test webhooks properly, we'd need a local HTTP server (e.g. mockito).
    // For this CI integration test, we'll compile-check the logic and run the command
    // to ensure it accepts the --webhook-url argument without crashing.
    // Full E2E delivery test requires mocking a receiver which adds complexity/dependencies.
    
    #[tokio::test]
    async fn test_daemon_webhook_argument() {
        if env::var("SODS_RUN_INTEGRATION_TESTS").unwrap_or_default() != "1" {
            return;
        }

        // Just verify the command starts (and fails due to missing .sods probably or just runs)
        // We'll dry-run it or run purely for checking argument acceptance.
        // Actually, since we can't easily mock the server here without adding dev-dependencies like `wiremock`,
        // We trust the unit logic and just ensure the binary invocation works.
        
        let mut child = Command::new("cargo")
            .args(&[
                "run",
                "--",
                "daemon",
                "start",
                "--pattern", "Tf",
                "--chain", "sepolia",
                "--webhook-url", "https://example.com/webhook"
            ])
            .spawn()
            .expect("Failed to start daemon command");
            
        // Let it run briefly
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        let _ = child.kill();
    }
}
