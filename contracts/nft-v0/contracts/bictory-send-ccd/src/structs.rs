use super::*;

/// The contract state.
#[contract_state(contract = "BictorySendCCD")]
#[derive(Serialize, SchemaType)]
pub struct State {
    pub send: SendCCD,
}

#[derive(Serialize, SchemaType, PartialEq, Eq)]
pub enum SendCCD {
    Send,
}
