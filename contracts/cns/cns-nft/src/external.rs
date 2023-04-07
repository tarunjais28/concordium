use commons_v1::Percentage;
use concordium_std::*;

#[derive(Serialize, SchemaType)]
pub struct InitParams {
    /// Address of a storage contract that stores token data.
    pub storage_contract: ContractAddress,
    /// Platform royalty that gets permanenty assigned to a token on mint.
    pub royalty_on_mint: Percentage,
    /// Grace period that gets permanenty assigned to a token on mint.
    pub grace_on_mint: Duration,
    /// Address that receives platform royalty.
    pub beneficiary: AccountAddress,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, SchemaType)]
pub enum UpdateOperation {
    Remove,
    Add,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, SchemaType)]
pub struct AddressUpdate {
    pub operation: UpdateOperation,
    pub address: ContractAddress,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, SchemaType)]
pub enum UpdateInternalValueParams {
    CnsContract(AddressUpdate),
    Royalty(Percentage),
    Beneficiary(AccountAddress),
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, SchemaType)]
pub enum ViewInternalValueParams {
    CnsContract(InternalAddressView),
    Royalty,
    Beneficiary,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, SchemaType)]
pub struct InternalAddressView {
    pub skip: u32,
    pub show: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, SchemaType)]
pub enum ViewInternalValueResult {
    CnsContract(Vec<ContractAddress>),
    Royalty(Percentage),
    Beneficiary(AccountAddress),
}
