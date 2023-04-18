use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub etf_token_code_id: u64,
    pub etf_token_name: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(Config)]
    GetConfig {},
}

#[cw_serde]
pub struct MigrateMsg {}


