use astroport::asset::AssetInfo;
use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, Response, StdError, Uint128};
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::state::{CONFIG, DEPOSIT_PAID_CACHE};

pub fn after_deposit(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    deposit: Uint128,
    sender: Addr,
    _basket_value: Uint128,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let config = CONFIG.load(deps.storage)?;
    let paid = DEPOSIT_PAID_CACHE.load(deps.storage)?;
    let leftover = deposit.checked_rem(paid)?;

    let mut messages: Vec<CosmosMsg<InjectiveMsgWrapper>> = vec![];

    if let AssetInfo::NativeToken { denom } = config.deposit_asset {
        let leftover_coins = Coin::new(u128::from(leftover), denom);

        let send_message = BankMsg::Send {
            to_address: sender.into_string(),
            amount: vec![leftover_coins],
        };

        messages.push(send_message.into());
    }

    Ok(Response::new()
        .add_attributes(vec![
            ("method", "after_deposit".to_string()),
            ("deposit", deposit.to_string()),
            ("paid", paid.to_string()),
        ])
        .add_messages(messages))
}
