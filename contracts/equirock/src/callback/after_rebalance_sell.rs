use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, DepsMut, Env, Response, StdError, Uint128, WasmMsg,
};

use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::{
    msg::ExecuteMsg,
    state::{ClobCache, CLOB_CACHE, CONFIG},
};

pub fn after_rebalance_sell(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let config = CONFIG.load(deps.storage)?;

    let clob_cache: Vec<ClobCache> = CLOB_CACHE.load(deps.storage)?;

    let received: Uint128 = clob_cache
        .to_owned()
        .into_iter()
        .try_fold(Uint128::zero(), |acc, c| {
            acc.checked_add((c.quantity * c.price - c.fee).sub(1).into())
        })?;

    let mut messages: Vec<CosmosMsg<InjectiveMsgWrapper>> = vec![];

    if let AssetInfo::NativeToken { denom } = &config.deposit_asset {
        if received.gt(&Uint128::zero()) {
            let received_coins = Coin::new(u128::from(received), denom);

            let deposit_msg = WasmMsg::Execute {
                contract_addr: env.contract.address.into_string(),
                msg: to_binary(&ExecuteMsg::Deposit {
                    asset: Asset {
                        amount: received,
                        info: config.deposit_asset,
                    },
                })?,
                funds: vec![received_coins],
            };

            messages.push(deposit_msg.into());
        }
    }

    Ok(Response::new()
        .add_attributes(vec![
            ("method", "after_rebalance_sell".to_string()),
            ("clob_cache", format!("{:?}", clob_cache)),
            ("received", received.to_string()),
        ])
        .add_messages(messages))
}
