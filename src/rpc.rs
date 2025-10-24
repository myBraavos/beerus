//! # RPC Server Module
//!
//! This module provides a JSON-RPC server implementation for Starknet operations.
//! It handles incoming RPC requests, routes them to the appropriate handlers,
//! and manages the execution context for function calls.
//!
//! ## Key Components
//!
//! - **Server**: Main RPC server implementation
//! - **Context**: RPC request context and handler
//! - **BlockResolver**: Resolves block IDs to concrete block references
//! - **Handler**: Request handling and routing logic
//!
//! ## Supported Operations
//!
//! The server supports all standard Starknet JSON-RPC methods including:
//! - Function calls (`starknet_call`)
//! - Block queries (`getBlock`, `getBlockNumber`, etc.)
//! - State queries (`getStorageAt`, `getNonce`, etc.)
//! - Transaction operations (`getTransaction`, `getTransactionReceipt`, etc.)
//! - Proof verification (`getProof`)

pub mod context;
pub mod handler;

use std::sync::Arc;

use axum::{
    routing::post, Router,
};

use crate::client::Client;
use crate::rpc::context::Context;
use crate::rpc::handler::handle_request;

/// RPC server for handling Starknet JSON-RPC requests
pub struct Server {
    client: Arc<Client<crate::client::http::Http>>,
}

impl Server {
    /// Create a new RPC server with the given client
    ///
    /// # Arguments
    ///
    /// * `client` - The Starknet client for handling requests
    pub fn new(client: Arc<Client<crate::client::http::Http>>) -> Self {
        Self { client }
    }

    /// Create the Axum router for the RPC server
    ///
    /// This method creates a new router with the RPC endpoint configured
    /// and the context set up for request handling.
    pub fn router(self) -> Router {
        let ctx = Context::new(self.client);
        Router::new()
            .route("/", post(handle_request))
            .with_state(ctx)
    }
}

/// Start the RPC server on the default address (0.0.0.0:3030)
///
/// # Arguments
///
/// * `server` - The configured RPC server
///
/// # Returns
///
/// Returns `Ok(())` if the server starts successfully, or an error if it fails.
pub async fn serve(server: Server) -> Result<(), Box<dyn std::error::Error>> {
    let app = server.router();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3030").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Start the RPC server on a specific address
///
/// # Arguments
///
/// * `server` - The configured RPC server
/// * `addr` - The address to bind to (e.g., "127.0.0.1:8080")
///
/// # Returns
///
/// Returns `Ok(())` if the server starts successfully, or an error if it fails.
pub async fn serve_on(
    server: Server,
    addr: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = server.router();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
