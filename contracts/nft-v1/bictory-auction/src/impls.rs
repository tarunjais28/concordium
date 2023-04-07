use super::*;

// Functions for creating, updating and querying the contract state.
impl<S: HasStateApi> State<S> {
    /// Creates a empty state with no tokens.
    pub fn empty(itm: Token, exp: Timestamp, state_builder: &mut StateBuilder<S>) -> Self {
        Self {
            viewable_state: ViewableState {
                auction_state: AuctionState::NotSoldYet,
                highest_bid: Amount::zero(),
                item: itm,
                expiry: exp,
                is_authorised: false,
            },
            bids: state_builder.new_map(),
        }
    }
}
