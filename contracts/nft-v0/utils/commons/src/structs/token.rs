use super::*;

/// Token
#[derive(Debug, Serialize, SchemaType, Hash, PartialEq, Eq, Clone)]
pub struct Token {
    pub contract: ContractAddress,
    pub id: ContractTokenId,
}
