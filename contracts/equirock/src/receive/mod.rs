use cosmwasm_std::{from_binary, DepsMut, Env, MessageInfo, Response, StdError, StdResult};

mod withdraw;

use cw20::Cw20ReceiveMsg;
use injective_cosmwasm::{InjectiveMsgWrapper, InjectiveQueryWrapper};
use withdraw::withdraw;

use crate::{msg::Cw20HookMsg, state::CONFIG};

pub fn receive(
    deps: DepsMut<InjectiveQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> StdResult<Response<InjectiveMsgWrapper>> {
    let config = CONFIG.load(deps.storage)?;

    if info.sender != deps.api.addr_humanize(&config.lp_token)? {
        return Err(StdError::GenericErr {
            msg: "Unauthorized".to_string(),
        });
    }

    match from_binary(&msg.msg)? {
        Cw20HookMsg::Withdraw {} => withdraw(deps, env, msg.sender, msg.amount),
    }
}
