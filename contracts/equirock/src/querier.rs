use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};
use cw20::{Cw20QueryMsg, TokenInfoResponse};
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
