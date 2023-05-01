use cosmwasm_std::{
    coins, to_binary, BankMsg, CosmosMsg, Decimal, DepsMut, Env, Response, StdResult, Uint128,
    WasmMsg,
};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, TokenInfoResponse};
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::state::CONFIG;

pub fn withdraw(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    sender: String,
    amount: Uint128,
) -> StdResult<Response<InjectiveMsgWrapper>> {
    let config = CONFIG.load(deps.storage)?;

    let sender = deps.api.addr_validate(&sender)?;

    // // calculate the size of vault and the amount of assets to withdraw
    // let collected_protocol_fees = COLLECTED_PROTOCOL_FEES.load(deps.storage)?;
    // let total_asset_amount = match &config.asset_info {
    //     AssetInfo::NativeToken { denom } => {
    //         deps.querier
    //             .query_balance(env.contract.address, denom)?
    //             .amount
    //     }
    //     AssetInfo::Token { contract_addr } => {
    //         let balance: BalanceResponse = deps.querier.query_wasm_smart(
    //             contract_addr,
    //             &Cw20QueryMsg::Balance {
    //                 address: env.contract.address.into_string(),
    //             },
    //         )?;
    //         balance.balance
    //     }
    // } // deduct protocol fees
    // .checked_sub(collected_protocol_fees.amount)?;

    // let total_share_amount: TokenInfoResponse = deps
    //     .querier
    //     .query_wasm_smart(config.liquidity_token.clone(), &Cw20QueryMsg::TokenInfo {})?;
    // let withdraw_amount =
    //     Decimal::from_ratio(amount, total_share_amount.total_supply) * total_asset_amount;

    // // create message to send back to user if cw20
    // let messages: Vec<CosmosMsg> = vec![
    //     match config.asset_info {
    //         AssetInfo::NativeToken { denom } => BankMsg::Send {
    //             to_address: sender.into_string(),
    //             amount: coins(withdraw_amount.u128(), denom),
    //         }
    //         .into(),
    //         AssetInfo::Token { contract_addr } => WasmMsg::Execute {
    //             contract_addr,
    //             msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //                 recipient: sender.into_string(),
    //                 amount: withdraw_amount,
    //             })?,
    //             funds: vec![],
    //         }
    //         .into(),
    //     },
    //     WasmMsg::Execute {
    //         contract_addr: config.liquidity_token.into_string(),
    //         funds: vec![],
    //         msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
    //     }
    //     .into(),
    // ];

    Ok(Response::new().add_attributes(vec![("method", "withdraw")]))
}
