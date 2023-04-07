use commons_v1::Bytes;
use concordium_cis1::TokenIdVec;
use concordium_std::*;

#[derive(Serialize, SchemaType)]
pub struct InitParams {
    pub registry: ContractAddress,
    pub nft: ContractAddress,
    pub price_oracle: ContractAddress,
    pub subscription_year_limit: u8,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct RegisterParams {
    pub domain: String,
    pub address: Address,
    pub duration_years: u8,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct ExtendParams {
    pub domain: String,
    pub duration_years: u8,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct SetAddressParams {
    pub domain: String,
    pub address: Address,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct ResolveParams {
    pub domain: String,
}

#[derive(Debug, Serialize, SchemaType, PartialEq, Eq)]
pub enum DataValue {
    Empty,
    Address(Address),
    Url(String),
    Binary(Bytes),
    String(String),
    Token(ContractAddress, TokenIdVec),
}

#[derive(Debug, Serialize, SchemaType)]
pub struct SetDataParams {
    pub domain: String,
    pub key: String,
    pub value: DataValue,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct GetDataParams {
    pub domain: String,
    pub key: String,
}

#[derive(Debug, Serialize, SchemaType)]
pub struct SubdomainParams {
    pub subdomain: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, SchemaType)]
pub enum InternalValue {
    CnsNft(ContractAddress),
    Oracle(ContractAddress),
    Beneficiary(AccountAddress),
    SubscriptionYearLimit(u8),
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, SchemaType)]
pub enum InternalViewParams {
    CnsNft,
    Oracle,
    Beneficiary,
    SubscriptionYearLimit,
}
