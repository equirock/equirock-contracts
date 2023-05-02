use astroport::asset::AssetInfo;
use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Response, StdError, Uint128};

use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::state::{ClobCache, CLOB_CACHE, CONFIG};

pub fn after_withdraw(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    sender: Addr,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let config = CONFIG.load(deps.storage)?;

    let clob_cache: ClobCache = CLOB_CACHE.load(deps.storage)?;
    let received: Uint128 = (clob_cache.quantity * clob_cache.price - clob_cache.fee).into();

    let mut messages: Vec<CosmosMsg<InjectiveMsgWrapper>> = vec![];

    if let AssetInfo::NativeToken { denom } = config.deposit_asset {
        if received.gt(&Uint128::zero()) {
            let received_coins = Coin::new(u128::from(received), denom);

            let send_message = BankMsg::Send {
                to_address: sender.clone().into_string(),
                amount: vec![received_coins],
            };

            messages.push(send_message.into());
        }
    }

    Ok(Response::new()
        .add_attributes(vec![
            ("method", "after_withdraw".to_string()),
            ("clob_cache", format!("{:?}", clob_cache)),
            ("received", received.to_string()),
        ])
        .add_messages(messages))
}
