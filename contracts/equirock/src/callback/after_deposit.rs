use astroport::asset::AssetInfo;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Response, StdError, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::{
    querier::query_token_info,
    state::{CONFIG, DEPOSIT_PAID_CACHE},
};

pub fn after_deposit(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    deposit: Uint128,
    sender: Addr,
    basket_value_before_deposit: Uint128,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let config = CONFIG.load(deps.storage)?;
    let paid: Uint128 = DEPOSIT_PAID_CACHE.load(deps.storage)?;
    let leftover = deposit.checked_rem(paid).unwrap_or(Uint128::zero());

    let mut messages: Vec<CosmosMsg<InjectiveMsgWrapper>> = vec![];

    if let AssetInfo::NativeToken { denom } = config.deposit_asset {
        if leftover.gt(&Uint128::zero()) {
            let leftover_coins = Coin::new(u128::from(leftover), denom);

            let send_message = BankMsg::Send {
                to_address: sender.clone().into_string(),
                amount: vec![leftover_coins],
            };

            messages.push(send_message.into());
        }
    }

    let liquidity_token = deps.api.addr_humanize(&config.lp_token)?;
    let total_share = query_token_info(&deps.querier, &liquidity_token)?.total_supply;

    let lp_amount = if total_share.is_zero() {
        paid
    } else {
        paid.checked_mul(total_share)?
            .checked_div(basket_value_before_deposit)?
    };

    messages.push(
        WasmMsg::Execute {
            contract_addr: liquidity_token.into_string(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: sender.into_string(),
                amount: lp_amount,
            })?,
            funds: vec![],
        }
        .into(),
    );

    Ok(Response::new()
        .add_attributes(vec![
            ("method", "after_deposit".to_string()),
            ("deposit", deposit.to_string()),
            ("paid", paid.to_string()),
            ("total_share", total_share.to_string()),
            (
                "basket_value_before_deposit",
                basket_value_before_deposit.to_string(),
            ),
            ("lp_amount", lp_amount.to_string()),
        ])
        .add_messages(messages))
}
