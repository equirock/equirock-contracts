use std::str::FromStr;

use cosmwasm_std::{DepsMut, Env, Reply, Response, StdError, StdResult};
use injective_cosmwasm::InjectiveMsgWrapper;
use injective_math::FPDecimal;
use protobuf::Message;

use crate::{response::MsgInstantiateContractResponse, state::CONFIG, ContractError};

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

    let api = deps.api;
    CONFIG.update(deps.storage, |mut config| -> StdResult<_> {
        config.lp_token = api.addr_canonicalize(&liquidity_token)?;
        Ok(config)
    })?;

    Ok(Response::new().add_attribute("liquidity_token_addr", liquidity_token))
}

pub fn handle_order(
    _deps: DepsMut,
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

    // let config = STATE.load(deps.storage)?;
    // let contract_address = env.contract.address;
    // let subaccount_id = config.contract_subaccount_id;

    // let cache = SWAP_OPERATION_STATE.load(deps.storage)?;

    // let purchased_coins = Coin::new(u128::from(quantity), config.base_denom.clone());
    // let paid = quantity * price + fee;
    // let leftover = cache.deposited_amount.amount - Uint128::from(u128::from(paid));
    // let leftover_coins = Coin::new(u128::from(leftover), config.quote_denom);
    // // we need to withdraw coins from subaccount to main account so we can transfer them back to a user
    // let withdraw_purchased_message = create_withdraw_msg(
    //     contract_address.clone(),
    //     subaccount_id.clone(),
    //     purchased_coins.clone(),
    // );
    // let withdraw_leftover_message =
    //     create_withdraw_msg(contract_address, subaccount_id, leftover_coins.clone());

    // let send_message = BankMsg::Send {
    //     to_address: cache.sender_address,
    //     amount: vec![purchased_coins, leftover_coins],
    // };

    // let response = Response::new()
    //     .add_message(withdraw_purchased_message)
    //     .add_message(withdraw_leftover_message)
    //     .add_message(send_message);

    Ok(Response::new()
        .add_attribute("quantity_foo_bar", quantity.to_string())
        .add_attribute("fee_foo_bar", fee.to_string())
        .add_attribute("price_foo_bar", price.to_string()))
}
