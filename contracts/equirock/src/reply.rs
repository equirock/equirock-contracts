use std::str::FromStr;

use cosmwasm_std::{DepsMut, Env, Reply, Response, StdError, StdResult};
use injective_cosmwasm::InjectiveMsgWrapper;
use injective_math::FPDecimal;
use protobuf::Message;

use crate::{
    response::MsgInstantiateContractResponse,
    state::{ClobCache, CLOB_CACHE, CONFIG},
    ContractError,
};

use injective_protobuf::proto::tx;

pub const INSTANTIATE_REPLY_ID: u64 = 1;
pub const ATOMIC_ORDER_REPLY_ID: u64 = 2;

pub fn handle_lp_init(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let data = msg.result.unwrap().data.unwrap();
    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(data.as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;
    let liquidity_token = res.address;

    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.lp_token = deps.api.addr_validate(&liquidity_token)?;
        Ok(config)
    })?;

    Ok(Response::new().add_attribute("liquidity_token_addr", liquidity_token))
}

pub fn handle_order(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response<InjectiveMsgWrapper>, ContractError> {
    let dec_scale_factor: FPDecimal = FPDecimal::from(1_000_000_000_000_000_000_i128);
    let id = msg.id;
    let order_response: tx::MsgCreateSpotMarketOrderResponse = Message::parse_from_bytes(
        msg.result
            .into_result()
            .map_err(ContractError::SubMsgFailure)?
            .data
            .ok_or_else(|| ContractError::ReplyParseFailure {
                id,
                err: "Missing reply data".to_owned(),
            })?
            .as_slice(),
    )
    .map_err(|err| ContractError::ReplyParseFailure {
        id,
        err: err.to_string(),
    })?;
    // unwrap results into trade_data
    let trade_data = match order_response.results.into_option() {
        Some(trade_data) => Ok(trade_data),
        None => Err(StdError::GenericErr {
            msg: "No trade data in order response".to_string(),
        }),
    }?;
    let quantity = FPDecimal::from_str(&trade_data.quantity)? / dec_scale_factor;
    let price = FPDecimal::from_str(&trade_data.price)? / dec_scale_factor;
    let fee = FPDecimal::from_str(&trade_data.fee)? / dec_scale_factor;

    CLOB_CACHE.update::<_, StdError>(deps.storage, |mut clob_cache| {
        clob_cache.push(ClobCache {
            quantity,
            price,
            fee,
        });

        Ok(clob_cache)
    })?;

    Ok(Response::new()
        .add_attribute("quantity", quantity.to_string())
        .add_attribute("fee", fee.to_string())
        .add_attribute("price", price.to_string()))
}
