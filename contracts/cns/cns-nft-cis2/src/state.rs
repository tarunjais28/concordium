use commons::{Authority, Percentage};
use concordium_cis2::{StandardIdentifierOwned, SupportResult};
use concordium_std::*;
use core::ops::DerefMut;

use crate::external::InitParams;

/// The contract state.
#[derive(Serial, DeserialWithState, StateClone)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    /// Addresses authorized to update and maintain the contract.
    pub authority: Authority<S>,
    /// The addresses of trusted CNS contracts.
    pub cns_contracts: StateSet<ContractAddress, S>,
    /// Operators for each address for this CNS NFT contract.
    pub operators: StateMap<Address, StateSet<Address, S>, S>,
    /// Platform royalty that gets permanenty assigned to a token on mint.
    pub royalty_on_mint: Percentage,
    /// Grace period that gets permanenty assigned to a token on mint.
    pub grace_on_mint: Duration,
    /// Address that receives platform royalty.
    pub beneficiary: AccountAddress,
    /// Storage contract address with all token data.
    pub token_storage: ContractAddress,
    /// Implementors.
    pub implementors: StateMap<StandardIdentifierOwned, Vec<ContractAddress>, S>,
}

impl<S: HasStateApi> State<S> {
    /// Creates a new state with no tokens.
    pub fn new(
        state_builder: &mut StateBuilder<S>,
        params: InitParams,
        origin: AccountAddress,
    ) -> Self {
        Self {
            authority: Authority::new(state_builder, Address::Account(origin)),
            cns_contracts: state_builder.new_set(),
            operators: state_builder.new_map(),
            royalty_on_mint: params.royalty_on_mint,
            grace_on_mint: params.grace_on_mint,
            beneficiary: params.beneficiary,
            token_storage: params.storage_contract,
            implementors: state_builder.new_map(),
        }
    }

    /// Add a new operator for the given address.
    ///
    /// Succeeds even if the `operator` is already an operator for the `owner`.
    pub fn add_operator(
        &mut self,
        owner: &Address,
        operator: &Address,
        state_builder: &mut StateBuilder<S>,
    ) {
        self.operators
            .entry(*owner)
            .or_insert_with(|| state_builder.new_set())
            .deref_mut()
            .insert(*operator);
    }

    /// Update the state removing an operator for a given address.
    /// Succeeds even if the `operator` is _not_ an operator for the `address`.
    pub fn remove_operator(&mut self, owner: &Address, operator: &Address) {
        self.operators
            .get_mut(owner)
            .map(|mut operators| operators.remove(operator));
    }

    /// Check if `address` is an operator for `owner`.
    pub fn is_operator(&self, owner: &Address, address: &Address) -> bool {
        self.operators
            .get(owner)
            .map(|operators| operators.contains(address))
            .unwrap_or(false)
            || owner == address
    }

    /// Check if `address` is an authorized CNS contract.
    pub fn is_authorized_cns_contract(&self, address: &Address) -> bool {
        match address {
            Address::Account(_) => false,
            Address::Contract(contract) => self.cns_contracts.contains(contract),
        }
    }

    /// Update the list of contracts implementing the specified standard.
    pub fn set_implementors(
        &mut self,
        id: StandardIdentifierOwned,
        contracts: Vec<ContractAddress>,
    ) {
        self.implementors.insert(id, contracts);
    }

    /// Update the list of contracts implementing the specified standard.
    pub fn get_implementors(&self, id: &StandardIdentifierOwned) -> SupportResult {
        if let Some(addresses) = self.implementors.get(id) {
            SupportResult::SupportBy(addresses.to_vec())
        } else {
            SupportResult::NoSupport
        }
    }
}
