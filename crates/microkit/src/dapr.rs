use anyhow::{Context, Result, bail};
use dapr::{Client, client::TonicClient, dapr::proto::runtime::v1::dapr_client::DaprClient};
use tonic::transport::Channel;

pub struct Dapr {
    pub client: Client<DaprClient<Channel>>,
}

impl Dapr {
    pub async fn new() -> Result<Self> {
        let endpoint = "https://127.0.0.1".to_string();
        log::debug!("Connecting to Dapr at: {}", endpoint);
        let client = match dapr::Client::<TonicClient>::connect(endpoint).await {
            Ok(client) => client,
            Err(e) => {
                if cfg!(debug_assertions) {
                    bail!("Dapr is not running. To run with Dapr, run: cargo mk all");
                }
                return Err(anyhow::anyhow!(e)).context("Dapr is not running");
            }
        };

        Ok(Self { client })
    }

    pub async fn get_secret(&mut self, secret_name: &str) -> Result<String> {
        let result = self.client.get_secret("secrets", secret_name).await?;
        let secret_opt = result.data.get(secret_name).cloned();
        secret_opt.ok_or_else(|| anyhow::anyhow!("Couldn't get secret"))
    }
}
