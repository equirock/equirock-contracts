use cosmwasm_std::{DepsMut, Env, Response, StdError, Uint128};
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

use crate::state::DEPOSIT_PAID_CACHE;

pub fn after_deposit(
    deps: DepsMut<InjectiveQueryWrapper>,
    _env: Env,
    deposit: Uint128,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    let paid = DEPOSIT_PAID_CACHE.load(deps.storage)?;

    Ok(Response::new().add_attributes(vec![
        ("method", "after_deposit".to_string()),
        ("deposit", deposit.to_string()),
        ("paid", paid.to_string()),
    ]))
}
