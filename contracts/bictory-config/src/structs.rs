use super::*;

/// The contract state.
#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    pub account: AccountAddress,
    pub phantom_data: PhantomData<S>,
}
