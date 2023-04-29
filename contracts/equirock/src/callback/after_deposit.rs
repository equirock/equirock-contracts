use cosmwasm_std::{DepsMut, Env, Response, StdError, Uint128};
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};

pub fn after_deposit(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    deposit: Uint128,
) -> Result<Response<InjectiveMsgWrapper>, StdError> {
    Ok(Response::new().add_attributes(vec![("method", "after_deposit".to_string())]))
}
