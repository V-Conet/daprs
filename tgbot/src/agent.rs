use reqwest::Client;

use crate::config::Config;

#[derive(Debug)]
pub struct AgentClient {
    client: Client,
    config: Config,
}

impl AgentClient {
    pub fn new(config: Config) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub fn nodes(&self) -> &[crate::config::NodeConfig] {
        &self.config.agent.nodes
    }

    pub async fn execute(&self, node: &str, cmd: shared::Cmd) -> Result<String, String> {
        let node_cfg = self
            .config
            .agent
            .nodes
            .iter()
            .find(|n| n.name == node)
            .ok_or_else(|| format!("Node '{}' not found", node))?;

        let url = format!("{}/cmd", node_cfg.address.trim_end_matches('/'));

        let resp = self
            .client
            .post(&url)
            .header("x-api-token", &self.config.tgbot.api_token)
            .json(&cmd)
            .timeout(std::time::Duration::from_secs(self.config.tgbot.timeout))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() {
            resp.text()
                .await
                .map_err(|e| format!("Failed to read response: {}", e))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("Error {}: {}", status, body))
        }
    }
}
