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

use std::sync::{Arc, RwLock};

use blockifier::execution::call_info::CallInfo;
use starknet_api::block::GasPrices;

use crate::{
    client::{rate_limiter::RateLimiter, settings::Settings, State},
    gen,
};

pub mod cache;
pub mod context;
pub mod contract_loader;
pub mod err;
pub mod executor;
pub mod map;
pub mod simulate;
pub mod utils;

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
/// * `rate_limiter` - The rate limiter for the L2 RPC calls
/// * `settings` - determines if verification is needed
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
    settings: Arc<RwLock<Settings>>,
) -> Result<CallInfo, Error> {
    let executor =
        executor::CallExecutor::new(client, state, rate_limiter, settings);
    executor.call(function_call)
}

pub fn simulate<T: gen::client::blocking::HttpClient + Clone>(
    client: gen::client::blocking::Client<T>,
    transactions: Vec<gen::BroadcastedTxn>,
    simulation_flags: Vec<gen::SimulationFlag>,
    state: State,
    gas_prices: &GasPrices,
    rate_limiter: RateLimiter,
    settings: Arc<RwLock<Settings>>,
) -> Result<Vec<gen::SimulatedTransaction>, Error> {
    let executor =
        executor::CallExecutor::new(client, state, rate_limiter, settings);
    executor.simulate(transactions, simulation_flags, gas_prices)
}

pub fn estimate_fee<T: gen::client::blocking::HttpClient + Clone>(
    client: gen::client::blocking::Client<T>,
    transactions: Vec<gen::BroadcastedTxn>,
    simulation_flags: Vec<gen::SimulationFlag>,
    state: State,
    gas_prices: &GasPrices,
    rate_limiter: RateLimiter,
    settings: Arc<RwLock<Settings>>,
) -> Result<Vec<gen::FeeEstimate>, Error> {
    let executor =
        executor::CallExecutor::new(client, state, rate_limiter, settings);
    executor.estimate_fee(transactions, simulation_flags, gas_prices)
}
