use crate::error::Error;
use crate::models::Token;
use crate::providers::EthereumProvider;
use alloy_primitives::Address;
use alloy_sol_types::sol;
use std::sync::Arc;

/// Define the ERC20 interface
sol! {
    #[sol(rpc)]
    interface IERC20 {
        function name() external view returns (string);
        function symbol() external view returns (string);
        function decimals() external view returns (uint8);
    }
}

/// Shared implementation of get_token for all DEX protocols
pub async fn get_token(
    provider: Arc<EthereumProvider>,
    token_address: Address,
    chain_id: u64,
) -> Result<Token, Error> {
    // Create contract instance
    let contract = IERC20::new(token_address, provider.provider());

    // Get name with fallback
    let name = match contract.name().call().await {
        Ok(name) => name,
        Err(_) => format!("Token-{}", token_address),
    };

    // Get symbol with fallback
    let symbol = match contract.symbol().call().await {
        Ok(symbol) => symbol,
        Err(_) => format!("TKN-{}", &token_address.to_string()[..6]),
    };

    // Get decimals with fallback
    let decimals = match contract.decimals().call().await {
        Ok(decimals) => decimals,
        Err(_) => 18u8,
    };

    Ok(Token {
        address: token_address,
        symbol,
        name,
        decimals,
        chain_id,
    })
}
