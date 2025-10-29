use std::fs;
use std::net::SocketAddr;

use eyre::{Context, Result};
use serde::Deserialize;
use validator::Validate;

/// Configuration constants
mod constants {
    pub const DEFAULT_POLL_SECS: u64 = 30;
    pub const DEFAULT_RPC_PORT: u16 = 3030;
    pub const MIN_POLL_SECS: u64 = 1;
    pub const MAX_POLL_SECS: u64 = 3600;
}

/// Environment variable names
mod env_vars {
    pub const STARKNET_RPC: &str = "STARKNET_RPC";
    pub const GATEWAY_URL: &str = "GATEWAY_URL";
    pub const DATABASE_URL: &str = "DATABASE_URL";
    pub const POLL_SECS: &str = "POLL_SECS";
    pub const RPC_ADDR: &str = "RPC_ADDR";
}

/// Server configuration containing both client and server settings
#[derive(Clone, Deserialize, Debug, Validate)]
pub struct ServerConfig {
    #[serde(flatten)]
    pub client: Config,
    #[serde(default = "default_poll_secs")]
    #[validate(range(
        min = "constants::MIN_POLL_SECS",
        max = "constants::MAX_POLL_SECS"
    ))]
    pub poll_secs: u64,
    #[serde(default = "default_rpc_addr")]
    pub rpc_addr: SocketAddr,
}

/// Client configuration for Starknet connection
#[derive(Clone, Deserialize, Debug, Validate)]
pub struct Config {
    #[validate(url)]
    pub starknet_rpc: String,
    #[validate(url)]
    pub gateway_url: String,
    #[cfg(not(target_arch = "wasm32"))]
    #[validate(url)]
    pub database_url: String,
}

impl ServerConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let poll_secs = Self::parse_poll_secs_from_env()?;
        let rpc_addr = Self::parse_rpc_addr_from_env()?;

        Ok(Self {
            client: Config {
                starknet_rpc: Self::parse_starknet_rpc_from_env()?,
                gateway_url: Self::parse_gateway_url_from_env()?,
                #[cfg(not(target_arch = "wasm32"))]
                database_url: Self::parse_database_url_from_env()?,
            },
            poll_secs,
            rpc_addr,
        })
    }

    /// Create configuration from TOML file
    pub fn from_file(path: &str) -> Result<Self> {
        let content =
            fs::read_to_string(path).context("Failed to read config file")?;
        let config: ServerConfig =
            toml::from_str(&content).context("Failed to parse config file")?;
        config.validate().context("Configuration validation failed")?;
        Ok(config)
    }

    /// Parse poll seconds from environment variable
    fn parse_poll_secs_from_env() -> Result<u64> {
        match std::env::var(env_vars::POLL_SECS) {
            Ok(value) => {
                let poll_secs =
                    value.parse().context("Invalid POLL_SECS value")?;
                if !(constants::MIN_POLL_SECS..=constants::MAX_POLL_SECS)
                    .contains(&poll_secs)
                {
                    eyre::bail!(
                        "POLL_SECS must be between {} and {}",
                        constants::MIN_POLL_SECS,
                        constants::MAX_POLL_SECS
                    );
                }
                Ok(poll_secs)
            }
            Err(_) => Ok(constants::DEFAULT_POLL_SECS),
        }
    }

    /// Parse RPC address from environment variable
    fn parse_rpc_addr_from_env() -> Result<SocketAddr> {
        match std::env::var(env_vars::RPC_ADDR) {
            Ok(value) => value.parse().context("Invalid RPC_ADDR format"),
            Err(_) => Ok(default_rpc_addr()),
        }
    }

    /// Parse Starknet RPC URL from environment variable
    fn parse_starknet_rpc_from_env() -> Result<String> {
        std::env::var(env_vars::STARKNET_RPC)
            .context("STARKNET_RPC environment variable is required")
    }

    /// Parse Gateway URL from environment variable
    fn parse_gateway_url_from_env() -> Result<String> {
        std::env::var(env_vars::GATEWAY_URL)
            .context("GATEWAY_URL environment variable is required")
    }

    /// Parse data directory from environment variable
    #[cfg(not(target_arch = "wasm32"))]
    fn parse_database_url_from_env() -> Result<String> {
        std::env::var(env_vars::DATABASE_URL)
            .context("DATABASE_URL environment variable is required")
    }
}

/// Default poll interval in seconds
fn default_poll_secs() -> u64 {
    constants::DEFAULT_POLL_SECS
}

/// Default RPC server address
fn default_rpc_addr() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], constants::DEFAULT_RPC_PORT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_starknet_rpc_url() {
        let config = ServerConfig {
            client: Config {
                starknet_rpc: "invalid-url".to_string(),
                gateway_url: "".to_string(),
                #[cfg(not(target_arch = "wasm32"))]
                database_url: "".to_string(),
            },
            poll_secs: 300,
            rpc_addr: SocketAddr::from(([0, 0, 0, 0], 3030)),
        };

        let result = config.client.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("starknet_rpc"));
    }

    #[test]
    fn test_invalid_poll_secs_range() {
        let config = ServerConfig {
            client: Config {
                starknet_rpc: "https://example.com".to_string(),
                gateway_url: "".to_string(),
                #[cfg(not(target_arch = "wasm32"))]
                database_url: "".to_string(),
            },
            poll_secs: 9999, // Too high
            rpc_addr: SocketAddr::from(([127, 0, 0, 1], 3030)),
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("poll_secs"));
    }

    #[test]
    fn test_valid_config() {
        let config = ServerConfig {
            client: Config {
                starknet_rpc: "https://example.com".to_string(),
                gateway_url: "".to_string(),
                #[cfg(not(target_arch = "wasm32"))]
                database_url: "".to_string(),
            },
            poll_secs: 300,
            rpc_addr: SocketAddr::from(([127, 0, 0, 1], 3030)),
        };

        let result = config.validate();
        assert!(result.is_ok());
    }
}
