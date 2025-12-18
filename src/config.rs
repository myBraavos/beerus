use std::fs;
use std::net::SocketAddr;

use eyre::{Context, Result};
use serde::Deserialize;
use validator::Validate;

/// Configuration constants
mod constants {
    pub const DEFAULT_POLL_SECS: u64 = 10;
    pub const DEFAULT_L1_POLL_SECS: u64 = 600; // 10 minutes
    pub const DEFAULT_RPC_PORT: u16 = 3030;
    pub const MIN_POLL_SECS: u64 = 1;
    pub const MAX_POLL_SECS: u64 = 3600;
    pub const MIN_L1_POLL_SECS: u64 = 30;
    pub const MAX_L1_POLL_SECS: u64 = 36000; // 10 hours
    pub const MIN_L2_RATE_LIMIT: u32 = 1;
    pub const MAX_L2_RATE_LIMIT: u32 = 1000;
    pub const DEFAULT_L2_RATE_LIMIT: u32 = 10;
    pub const MIN_L1_RANGE_BLOCKS: u64 = 1;
    pub const MAX_L1_RANGE_BLOCKS: u64 = 100000;
    pub const DEFAULT_L1_RANGE_BLOCKS: u64 = 9;
}

/// Environment variable names
mod env_vars {
    pub const ETH_RPC: &str = "ETH_RPC";
    pub const STARKNET_RPC: &str = "STARKNET_RPC";
    pub const GATEWAY_URL: &str = "GATEWAY_URL";
    pub const DATABASE_URL: &str = "DATABASE_URL";
    pub const DISABLE_BACKGROUND_LOADER: &str = "DISABLE_BACKGROUND_LOADER";
    pub const VALIDATE_HISTORICAL_BLOCKS: &str = "VALIDATE_HISTORICAL_BLOCKS";
    pub const POLL_SECS: &str = "POLL_SECS";
    pub const L1_POLL_SECS: &str = "L1_POLL_SECS";
    pub const RPC_ADDR: &str = "RPC_ADDR";
    pub const L2_RATE_LIMIT: &str = "L2_RATE_LIMIT";
    pub const L1_RANGE_BLOCKS: &str = "L1_RANGE_BLOCKS";
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
    #[serde(default = "default_l1_poll_secs")]
    #[validate(range(
        min = "constants::MIN_L1_POLL_SECS",
        max = "constants::MAX_L1_POLL_SECS"
    ))]
    pub l1_poll_secs: u64,
    #[serde(default = "default_rpc_addr")]
    pub rpc_addr: SocketAddr,
}

/// Client configuration for Starknet connection
#[derive(Clone, Deserialize, Debug, Validate)]
pub struct Config {
    #[validate(url)]
    pub eth_rpc: String,
    #[validate(url)]
    pub starknet_rpc: String,
    #[validate(url)]
    pub gateway_url: String,
    #[serde(default = "default_batch_size")]
    #[validate(range(
        min = "constants::MIN_L2_RATE_LIMIT",
        max = "constants::MAX_L2_RATE_LIMIT"
    ))]
    pub l2_rate_limit: u32, // requests per second
    #[serde(default = "default_l1_range_blocks")]
    #[validate(range(
        min = "constants::MIN_L1_RANGE_BLOCKS",
        max = "constants::MAX_L1_RANGE_BLOCKS"
    ))]
    pub l1_range_blocks: u64,
    #[cfg(not(target_arch = "wasm32"))]
    #[validate(url)]
    pub database_url: String,
    #[serde(default = "default_disable_background_loader")]
    pub disable_background_loader: bool,
    #[serde(default = "default_validate_historical_blocks")]
    pub validate_historical_blocks: bool,
}

impl ServerConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        let poll_secs = Self::parse_poll_secs_from_env()?;
        let l1_poll_secs = Self::parse_l1_poll_secs_from_env()?;
        let rpc_addr = Self::parse_rpc_addr_from_env()?;

        Ok(Self {
            client: Config {
                eth_rpc: Self::parse_eth_rpc_from_env()?,
                starknet_rpc: Self::parse_starknet_rpc_from_env()?,
                gateway_url: Self::parse_gateway_url_from_env()?,
                l2_rate_limit: Self::parse_batch_size_from_env()?,
                l1_range_blocks: Self::parse_l1_range_blocks_from_env()?,
                #[cfg(not(target_arch = "wasm32"))]
                database_url: Self::parse_database_url_from_env()?,
                disable_background_loader:
                    Self::parse_disable_background_loader_from_env()?,
                validate_historical_blocks:
                    Self::parse_validate_historical_blocks_from_env()?,
            },
            poll_secs,
            l1_poll_secs,
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
        parse_env_range(
            env_vars::POLL_SECS,
            constants::MIN_POLL_SECS,
            constants::MAX_POLL_SECS,
            constants::DEFAULT_POLL_SECS,
        )
    }

    /// Parse L1 poll seconds from environment variable
    fn parse_l1_poll_secs_from_env() -> Result<u64> {
        parse_env_range(
            env_vars::L1_POLL_SECS,
            constants::MIN_L1_POLL_SECS,
            constants::MAX_L1_POLL_SECS,
            constants::DEFAULT_L1_POLL_SECS,
        )
    }

    /// Parse batch size from environment variable
    fn parse_batch_size_from_env() -> Result<u32> {
        parse_env_range(
            env_vars::L2_RATE_LIMIT,
            constants::MIN_L2_RATE_LIMIT,
            constants::MAX_L2_RATE_LIMIT,
            constants::DEFAULT_L2_RATE_LIMIT,
        )
    }

    /// Parse L1 range blocks from environment variable
    fn parse_l1_range_blocks_from_env() -> Result<u64> {
        parse_env_range(
            env_vars::L1_RANGE_BLOCKS,
            constants::MIN_L1_RANGE_BLOCKS,
            constants::MAX_L1_RANGE_BLOCKS,
            constants::DEFAULT_L1_RANGE_BLOCKS,
        )
    }

    /// Parse RPC address from environment variable
    fn parse_rpc_addr_from_env() -> Result<SocketAddr> {
        match std::env::var(env_vars::RPC_ADDR) {
            Ok(value) => value.parse().context("Invalid RPC_ADDR format"),
            Err(_) => Ok(default_rpc_addr()),
        }
    }

    /// Parse ETH RPC URL from environment variable
    fn parse_eth_rpc_from_env() -> Result<String> {
        std::env::var(env_vars::ETH_RPC)
            .context("ETH_RPC environment variable is required")
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

    /// Parse disable background loader from environment variable
    fn parse_disable_background_loader_from_env() -> Result<bool> {
        match std::env::var(env_vars::DISABLE_BACKGROUND_LOADER) {
            Ok(value) => Ok(value.to_lowercase() == "true" || value == "1"),
            Err(_) => Ok(default_disable_background_loader()),
        }
    }

    /// Parse validate historical blocks from environment variable
    fn parse_validate_historical_blocks_from_env() -> Result<bool> {
        match std::env::var(env_vars::VALIDATE_HISTORICAL_BLOCKS) {
            Ok(value) => Ok(value.to_lowercase() == "true" || value == "1"),
            Err(_) => Ok(default_validate_historical_blocks()),
        }
    }
}

/// Default poll interval in seconds
fn default_poll_secs() -> u64 {
    constants::DEFAULT_POLL_SECS
}

/// Default L1 poll interval in seconds
fn default_l1_poll_secs() -> u64 {
    constants::DEFAULT_L1_POLL_SECS
}

/// Default batch size
fn default_batch_size() -> u32 {
    constants::DEFAULT_L2_RATE_LIMIT
}

/// Default L1 range blocks
fn default_l1_range_blocks() -> u64 {
    constants::DEFAULT_L1_RANGE_BLOCKS
}

/// Default disable background loader
fn default_disable_background_loader() -> bool {
    false
}

/// Default validate historical blocks
fn default_validate_historical_blocks() -> bool {
    false
}
/// Default RPC server address
fn default_rpc_addr() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], constants::DEFAULT_RPC_PORT))
}

fn parse_env_range<T: std::str::FromStr + PartialOrd + std::fmt::Display>(
    env_var: &str,
    min: T,
    max: T,
    default: T,
) -> Result<T> {
    match std::env::var(env_var) {
        Ok(value) => {
            let parsed_value = value.parse().map_err(|_| {
                eyre::eyre!("Invalid {} value: {}", env_var, value)
            })?;
            if !(&min..=&max).contains(&&parsed_value) {
                eyre::bail!("{} must be between {} and {}", env_var, min, max);
            }
            Ok(parsed_value)
        }
        Err(_) => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_starknet_rpc_url() {
        let config = ServerConfig {
            client: Config {
                starknet_rpc: "invalid-url".to_string(),
                eth_rpc: "".to_string(),
                gateway_url: "".to_string(),
                l2_rate_limit: 10,
                l1_range_blocks: 9,
                #[cfg(not(target_arch = "wasm32"))]
                database_url: "".to_string(),
                disable_background_loader: false,
                validate_historical_blocks: false,
            },
            poll_secs: 300,
            l1_poll_secs: 600,
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
                eth_rpc: "".to_string(),
                gateway_url: "".to_string(),
                l2_rate_limit: 10,
                l1_range_blocks: 9,
                #[cfg(not(target_arch = "wasm32"))]
                database_url: "".to_string(),
                disable_background_loader: false,
                validate_historical_blocks: false,
            },
            poll_secs: 9999, // Too high
            l1_poll_secs: 600,
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
                eth_rpc: "".to_string(),
                gateway_url: "".to_string(),
                l2_rate_limit: 10,
                l1_range_blocks: 9,
                #[cfg(not(target_arch = "wasm32"))]
                database_url: "".to_string(),
                disable_background_loader: false,
                validate_historical_blocks: false,
            },
            poll_secs: 300,
            l1_poll_secs: 600,
            rpc_addr: SocketAddr::from(([127, 0, 0, 1], 3030)),
        };

        let result = config.validate();
        assert!(result.is_ok());
    }
}
