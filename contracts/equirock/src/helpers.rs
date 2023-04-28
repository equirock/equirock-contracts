use injective_cosmwasm::InjectiveMsgWrapper;
use injective_math::FPDecimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, CustomQuery, Querier, QuerierWrapper, StdResult, SubMsg, WasmMsg,
    WasmQuery,
};

use crate::{
    msg::{ExecuteMsg, QueryMsg},
    state::Config,
};

/// CwTemplateContract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct CwTemplateContract(pub Addr);

impl CwTemplateContract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    pub fn config<Q, T, CQ>(&self, querier: &Q) -> StdResult<Config>
    where
        Q: Querier,
        T: Into<String>,
        CQ: CustomQuery,
    {
        let msg = QueryMsg::GetConfig {};
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        let res: Config = QuerierWrapper::<CQ>::new(querier).query(&query)?;
        Ok(res)
    }
}

pub fn i32_to_dec(source: i32) -> FPDecimal {
    FPDecimal::from(i128::from(source))
}

pub fn get_message_data(
    response: &[SubMsg<InjectiveMsgWrapper>],
    position: usize,
) -> &InjectiveMsgWrapper {
    let sth = match &response.get(position).unwrap().msg {
        CosmosMsg::Custom(msg) => msg,
        _ => panic!("No wrapped message found"),
    };
    sth
}
