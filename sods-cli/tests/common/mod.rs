use assert_cmd::Command;
use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path}};
use tempfile::TempDir;

pub struct TestEnv {
    pub server: MockServer,
    pub home_dir: TempDir,
}

impl TestEnv {
    pub async fn new() -> Self {
        Self {
            server: MockServer::start().await,
            home_dir: TempDir::new().unwrap(),
        }
    }

    pub fn sods(&self) -> Command {
        let mut cmd = Command::cargo_bin("sods").unwrap();
        let path = self.home_dir.path();
        cmd.env("HOME", path);
        cmd.env("USERPROFILE", path);
        cmd.env("APPDATA", path);
        cmd.env("LOCALAPPDATA", path);
        cmd
    }

    pub async fn mock_rpc(&self, method_name: &str, response_json: serde_json::Value) {
        let response = ResponseTemplate::new(200)
            .set_body_json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": response_json
            }));

        Mock::given(method("POST"))
            .and(path("/"))
            // We use a body matcher here if we wanted to be specific about the JSON-RPC method,
            // but for simplicity in mocks, we can just respond to all POSTs or use a header/filter.
            .respond_with(response)
            .mount(&self.server)
            .await;
    }
}

pub fn get_mock_block_header(block_number: u64) -> serde_json::Value {
    serde_json::json!({
        "number": format!("0x{:x}", block_number),
        "hash": "0x1234567890123456789012345678901234567890123456789012345678901234",
        "parentHash": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "receiptsRoot": "0x0000000000000000000000000000000000000000000000000000000000000000",
        "timestamp": "0x65b17a10",
        "transactions": []
    })
}
