//! # Execution Module
//!
//! This module provides functionality for executing Starknet function calls and managing
//! execution contexts. It handles the interaction between the generated RPC client and
//! the blockifier execution engine.
//!
//! ## Key Components
//!
//! - **CallExecutor**: Executes function calls using blockifier
//! - **StateProxy**: Adapter between RPC client and blockifier state interface
//! - **ContractLoader**: Handles loading and conversion of contract classes
//! - **ExecutionContextBuilder**: Creates execution contexts for function calls
//! - **Type Mappings**: Conversion utilities between generated and Cairo types

use blockifier::execution::call_info::CallInfo;

use crate::{
    client::{rate_limiter::RateLimiter, State},
    gen,
};

pub mod cache;
pub mod context;
pub mod contract_loader;
pub mod err;
pub mod executor;
pub mod map;
pub mod state_proxy;

use err::Error;

/// Execute a function call on the Starknet state
///
/// This function creates a call executor and executes the provided function call
/// against the given state. It handles the conversion between the RPC client
/// and the blockifier execution engine.
///
/// # Arguments
///
/// * `client` - The blocking HTTP client for RPC calls
/// * `function_call` - The function call to execute
/// * `state` - The current blockchain state
///
/// # Returns
///
/// Returns the execution result containing call information and execution details.
///
/// # Errors
///
/// This function can return various execution errors including:
/// - Contract loading failures
/// - State access errors
/// - Execution errors from blockifier
pub fn call<T: gen::client::blocking::HttpClient + Clone>(
    client: gen::client::blocking::Client<T>,
    function_call: gen::FunctionCall,
    state: State,
    rate_limiter: RateLimiter,
) -> Result<CallInfo, Error> {
    let executor = executor::CallExecutor::new(client, state, rate_limiter);
    executor.execute(function_call)
}
