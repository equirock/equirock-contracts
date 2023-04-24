use cosmwasm_std::{DepsMut, Env, Reply, Response, StdError, StdResult};
use injective_cosmwasm::InjectiveMsgWrapper;
use protobuf::Message;

use crate::{response::MsgInstantiateContractResponse, state::CONFIG, ContractError};

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
