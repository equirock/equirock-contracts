use astroport::asset::AssetInfo;
use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, Uint128, WasmQuery};
use cw20::{BalanceResponse as Cw20BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use injective_cosmwasm::InjectiveQueryWrapper;
use pyth_sdk_cw::{PriceFeedResponse, PriceIdentifier, QueryMsg};

pub fn query_token_info(
    querier: &QuerierWrapper<InjectiveQueryWrapper>,
    contract_addr: Addr,
) -> StdResult<TokenInfoResponse> {
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(token_info)
}

pub fn query_native_balance(
    querier: &QuerierWrapper<InjectiveQueryWrapper>,
    account_addr: impl Into<String>,
    denom: impl Into<String>,
) -> StdResult<Uint128> {
    querier
        .query_balance(account_addr, denom)
        .map(|coin| coin.amount)
}

pub fn query_token_balance(
    querier: &QuerierWrapper<InjectiveQueryWrapper>,
    contract_addr: impl Into<String>,
    account_addr: impl Into<String>,
) -> StdResult<Uint128> {
    // load balance from the token contract
    let resp: Cw20BalanceResponse = querier
        .query_wasm_smart(
            contract_addr,
            &Cw20QueryMsg::Balance {
                address: account_addr.into(),
            },
        )
        .unwrap_or_else(|_| Cw20BalanceResponse {
            balance: Uint128::zero(),
        });

    Ok(resp.balance)
}

pub fn query_balance(
    querier: &QuerierWrapper<InjectiveQueryWrapper>,
    asset: &AssetInfo,
    account_addr: &Addr,
) -> StdResult<Uint128> {
    match asset {
        AssetInfo::Token { contract_addr, .. } => {
            query_token_balance(querier, contract_addr, account_addr)
        }
        AssetInfo::NativeToken { denom } => query_native_balance(querier, account_addr, denom),
    }
}

pub fn query_price_feed(
    querier: &QuerierWrapper<InjectiveQueryWrapper>,
    contract_addr: Addr,
    id: PriceIdentifier,
) -> StdResult<PriceFeedResponse> {
    let price_feed_response = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.into_string(),
        msg: to_binary(&QueryMsg::PriceFeed { id })?,
    }))?;
    Ok(price_feed_response)
}
