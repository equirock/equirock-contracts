use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};
use cw20::{Cw20QueryMsg, TokenInfoResponse};

pub fn query_token_info(
    querier: &QuerierWrapper,
    contract_addr: Addr,
) -> StdResult<TokenInfoResponse> {
    let token_info: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(token_info)
}
