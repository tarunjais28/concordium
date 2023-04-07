use commons::{Authority, Bytes};
use concordium_std::*;

#[derive(Debug, Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    /// Addresses that are allowed to modify storage data
    pub writers: StateSet<Address, S>,
    pub authority: Authority<S>,
    /// Key-value data storage
    pub storage: StateMap<Bytes, StateMap<Bytes, Bytes, S>, S>,
}

impl<S: HasStateApi> State<S> {
    pub fn new(state_builder: &mut StateBuilder<S>, admin: AccountAddress) -> Self {
        let authority = Authority::new(state_builder, Address::Account(admin));
        Self {
            writers: state_builder.new_set(),
            authority,
            storage: state_builder.new_map(),
        }
    }

    pub fn has_writer_rights(&self, addr: &Address) -> bool {
        self.writers.contains(addr)
    }
}
