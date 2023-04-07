use super::*;

/// The contract state.
#[contract_state(contract = "BictoryConfig")]
#[derive(Serialize, SchemaType)]
pub struct State {
    pub account: AccountAddress,
}
