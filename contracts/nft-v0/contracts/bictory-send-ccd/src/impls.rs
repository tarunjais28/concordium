use super::*;

// Functions for creating and updating the contract state.
impl State {
    /// Creates a new state with Send.
    pub fn empty() -> Self {
        State {
            send: SendCCD::Send,
        }
    }
}
