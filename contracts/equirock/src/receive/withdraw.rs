use astroport::asset::AssetInfo;
use cosmwasm_std::{
    to_binary, CosmosMsg, Decimal, DepsMut, Env, Response, StdError, StdResult, SubMsg, Uint128,
    WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, TokenInfoResponse};
use injective_cosmwasm::{
    get_default_subaccount_id_for_checked_address, InjectiveMsgWrapper, InjectiveQuerier,
    InjectiveQueryWrapper, OrderType,
};

use crate::{
    execute::spot_order,
    msg::{CallbackMsg, ExecuteMsg},
    querier::{query_balance, query_decimals},
    query::{basket_asset_price, pyth_price},
    reply::ATOMIC_ORDER_REPLY_ID,
    state::{BASKET, CONFIG},
};

pub fn withdraw(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    sender: String,
    amount: Uint128,
) -> StdResult<Response<InjectiveMsgWrapper>> {
    let config = CONFIG.load(deps.storage)?;

    let sender = deps.api.addr_validate(&sender)?;
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

    let total_share: TokenInfoResponse = deps.querier.query_wasm_smart(
        deps.api.addr_humanize(&config.lp_token)?,
        &Cw20QueryMsg::TokenInfo {},
    )?;

    let basket = BASKET.load(deps.storage)?;
    let withdraw_ratio = Decimal::from_ratio(amount, total_share.total_supply);

    let contract = &env.contract.address;
    let subaccount_id = get_default_subaccount_id_for_checked_address(contract);
    let slippage = Decimal::one().checked_rem(Decimal::from_ratio(1u128, 100u128))?;
    let mut submessages: Vec<SubMsg<InjectiveMsgWrapper>> = vec![];
    let injective_querier = InjectiveQuerier::new(&deps.querier);

    let mut log: Vec<String> = vec![];
    for basket_asset in basket.assets {
        let market = injective_querier.query_spot_market(&basket_asset.spot_market_id)?;
        if let Some(market) = market.market {
            log.push(format!("{:?}", market));
            let base_decimals = query_decimals(
                &deps.querier,
                &AssetInfo::NativeToken {
                    denom: market.base_denom.to_owned(),
                },
            );
            let quote_decimals = 6u64; // USDT

            log.push(format!("base_decimals {:?}", base_decimals));

            let fetch_price = basket_asset_price(
                &deps.querier,
                &env,
                &config.pyth_contract_addr,
                basket_asset.pyth_price_feed,
            )?;
            let price = pyth_price(fetch_price.current_price)?;

            let amount = query_balance(
                &deps.querier,
                &basket_asset.asset.info,
                &env.contract.address,
            )?;

            let order_msg = spot_order(
                slippage,
                price,
                withdraw_ratio.checked_mul(
                    Decimal::from_atomics(amount, base_decimals as u32)
                        .map_err(|e| StdError::generic_err(e.to_string()))?,
                    // .checked_mul(Decimal::from_str("0.998")?)?,
                )?,
                &market,
                base_decimals,
                quote_decimals,
                OrderType::SellAtomic,
                &subaccount_id,
                contract,
            )?;

            log.push(format!("order_msg {:?}", order_msg));

            let order_message = SubMsg::reply_on_success(order_msg, ATOMIC_ORDER_REPLY_ID);
            submessages.push(order_message);
        }
    }

    let after_deposit_msg = WasmMsg::Execute {
        contract_addr: contract.to_owned().into_string(),
        msg: to_binary(&ExecuteMsg::Callback(CallbackMsg::AfterWithdraw { sender }))?,
        funds: vec![],
    };

    let burn_lp_tokens_msg = WasmMsg::Execute {
        contract_addr: deps.api.addr_humanize(&config.lp_token)?.into_string(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
    };

    let messages: Vec<CosmosMsg<InjectiveMsgWrapper>> =
        vec![after_deposit_msg.into(), burn_lp_tokens_msg.into()];

    Ok(Response::new()
        .add_attributes(vec![("method", "withdraw")])
        .add_messages(messages))
}
