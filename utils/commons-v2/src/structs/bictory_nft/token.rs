use super::*;

/// Token
#[derive(Debug, Serialize, SchemaType, Hash, PartialEq, Eq, Clone)]
pub struct Token {
    pub contract: ContractAddress,
    pub id: ContractTokenId,
}

#[derive(Serialize, SchemaType)]
pub struct ViewTokenParams {
    /// Token Data
    pub owner: Address,
    /// The address which are currently enabled as operators for this address.
    pub token_id: ContractTokenId,
}
