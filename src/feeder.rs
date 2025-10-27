use eyre::{Context, OptionExt, Result};

use crate::{client::{state::GatewayState, State}, r#gen::{BlockId, Felt}};

/// Gateway client for interacting with Starknet feeder gateway
pub struct GatewayClient {
    url: String,
    client: reqwest::Client,
}

impl GatewayClient {
    /// Create a new gateway client
    pub fn new(url: &str) -> Result<Self> {
        if url.ends_with('/') {
            eyre::bail!("Gateway URL must not end with '/'.");
        }
        Ok(Self {
            url: url.to_owned(),
            client: reqwest::Client::new()
        })
    }

    /// Get public key for a block
    pub async fn get_pubkey(&self, block_hash: &str) -> Result<String> {
        let url = self.build_url("/feeder_gateway/get_public_key", &[("blockHash", block_hash)]);
        let response = self.make_get_request(&url).await?;
        Ok(response)
    }

    /// Get signature for a block
    pub async fn get_signature(&self, block_hash: &str) -> Result<(String, String)> {
        let url = self.build_url("/feeder_gateway/get_signature", &[("blockHash", block_hash)]);
        let json = self.make_json_request(&url).await?;

        self.validate_and_extract_signature(&json, block_hash)
    }

    /// Get current state from the latest block
    pub async fn get_state(&self, block_number: BlockId) -> Result<GatewayState> {
        // Own the strings so we don't create short-lived temporaries
        let mut params_owned: Vec<(String, String)> = vec![
            ("headerOnly".to_string(), "true".to_string()),
        ];

        if let BlockId::BlockNumber { block_number } = block_number {
            params_owned.push((
                "blockNumber".to_string(),
                block_number.0.to_string(),
            ));
        } else {
            params_owned.push((
                "blockNumber".to_string(),
                "latest".to_string(),
            ));
        }

        // Build a temporary vector of &str pairs that borrow from params_owned.
        // params_owned must stay alive while we use params_refs (it does here).
        let params_refs: Vec<(&str, &str)> = params_owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let url = self.build_url("/feeder_gateway/get_block", &params_refs);
        let json = self.make_json_request(&url).await?;

        self.validate_and_extract_state(&json)
    }

    /// Build URL with query parameters
    fn build_url(&self, path: &str, params: &[(&str, &str)]) -> String {
        let mut url = format!("{}{}", self.url, path);
        if !params.is_empty() {
            url.push('?');
            let query_string = params
                .iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<_>>()
                .join("&");
            url.push_str(&query_string);
        }
        url
    }

    /// Make a GET request and return text response
    async fn make_get_request(&self, url: &str) -> Result<String> {
        self.client
            .get(url)
            .send()
            .await
            .context("Failed to send gateway request")?
            .text()
            .await
            .context("Failed to receive gateway response")
    }

    /// Make a GET request and return JSON response
    async fn make_json_request(&self, url: &str) -> Result<serde_json::Value> {
        self.client
            .get(url)
            .send()
            .await
            .context("Failed to send gateway request")?
            .json()
            .await
            .context("Failed to receive gateway response")
    }

    /// Validate and extract signature from JSON response
    fn validate_and_extract_signature(
        &self,
        json: &serde_json::Value,
        expected_block_hash: &str,
    ) -> Result<(String, String)> {
        let hash = json["block_hash"]
            .as_str()
            .ok_or_eyre("Gateway: invalid block hash")?;

        if hash != expected_block_hash {
            eyre::bail!("Gateway: block hash mismatch");
        }

        let signature = json["signature"]
            .as_array()
            .ok_or_eyre("Gateway: invalid signature format")?;

        if signature.len() != 2 {
            eyre::bail!("Gateway: signature must have exactly 2 components");
        }

        let r = signature[0]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_eyre("Gateway: invalid signature component r")?;
        let s = signature[1]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_eyre("Gateway: invalid signature component s")?;

        Ok((r, s))
    }

    /// Validate and extract state from JSON response
    fn validate_and_extract_state(&self, json: &serde_json::Value) -> Result<GatewayState> {
        let block_number: u64 = json["block_number"]
            .as_u64()
            .ok_or_eyre("Gateway: missing or invalid block_number")?;

        let block_hash = json["block_hash"]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_eyre("Gateway: missing or invalid block_hash")?;

        Ok(GatewayState {
            block_number,
            block_hash: Felt::try_new(&block_hash)
                .context("Invalid block hash format")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use wiremock::{
        matchers::{method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    #[tokio::test]
    async fn test_get_state_success() -> Result<()> {
        const BLOCK_NUMBER: u64 = 1056427;
        const BLOCK_HASH: &str =
            "0x7c7b366f1b31a556ace49e1affe3b4ed3cfb5aa328b85307655ea70dadd0cc6";
        const STATE_ROOT: &str =
            "0x33d912445ba4f73ce6d910f3952e722aef1c55ee81278b3039b50243278f561";

        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feeder_gateway/get_block"))
            .and(query_param("blockNumber", "latest"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "block_number": BLOCK_NUMBER,
                    "block_hash": BLOCK_HASH,
                    "state_root": STATE_ROOT,
                    "status": "ACCEPTED_ON_L2"
                }),
            ))
            .mount(&mock)
            .await;

        let gateway = GatewayClient::new(mock.uri().as_str())?;
        let state = gateway.get_state(BlockId::BlockTag(crate::gen::BlockTag::Latest)).await?;

        assert_eq!(state.block_number, BLOCK_NUMBER);
        assert_eq!(state.block_hash.as_ref(), BLOCK_HASH);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_signature_success() -> Result<()> {
        const BLOCK_HASH: &str = "0x1234567890abcdef";
        const R: &str = "0xabcdef1234567890";
        const S: &str = "0x9876543210fedcba";

        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feeder_gateway/get_signature"))
            .and(query_param("blockHash", BLOCK_HASH))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({
                    "block_hash": BLOCK_HASH,
                    "signature": [R, S]
                }),
            ))
            .mount(&mock)
            .await;

        let gateway = GatewayClient::new(mock.uri().as_str())?;
        let (r, s) = gateway.get_signature(BLOCK_HASH).await?;

        assert_eq!(r, R);
        assert_eq!(s, S);
        Ok(())
    }

    #[tokio::test]
    async fn test_get_pubkey_success() -> Result<()> {
        const BLOCK_HASH: &str = "0x1234567890abcdef";
        const PUBKEY: &str = "0xabcdef1234567890";

        let mock = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/feeder_gateway/get_public_key"))
            .and(query_param("blockHash", BLOCK_HASH))
            .respond_with(ResponseTemplate::new(200).set_body_string(PUBKEY))
            .mount(&mock)
            .await;

        let gateway = GatewayClient::new(mock.uri().as_str())?;
        let pubkey = gateway.get_pubkey(BLOCK_HASH).await?;

        assert_eq!(pubkey, PUBKEY);
        Ok(())
    }
}
