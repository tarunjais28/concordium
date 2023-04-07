use super::*;

/// Minting Data.
#[derive(Serialize, SchemaType, Clone, Eq, PartialEq, concordium_std::hash::Hash)]
pub struct CnsMintParams {
    /// TokenId to mint
    pub token_id: ContractTokenId,
    /// Token domain name
    pub domain: String,
    /// The account address of owner.
    pub owner: Address,
    /// Initial subscription duration of the CNS Domain.
    pub duration: Duration,
}

#[derive(Serialize, SchemaType)]
pub struct LendParams {
    /// TokenId to lend
    pub token_id: ContractTokenId,
    /// Extention duration of the CNS Domain.
    pub extension: Duration,
}

#[derive(Serialize, SchemaType)]
pub struct TokenParams {
    /// Token ID.
    pub token_id: ContractTokenId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, SchemaType)]
pub struct TokenInfo {
    pub domain: String,
    pub royalty: Percentage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, SchemaType)]
pub struct TokenSubscriptionStatus {
    pub owner: Address,
    pub expiry: SubscriptionExpiryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, SchemaType)]
pub enum SubscriptionExpiryStatus {
    Owned(Timestamp),
    Grace(Timestamp),
    Expired,
}

impl TokenSubscriptionStatus {
    pub fn is_owned_by(&self, address: Address) -> bool {
        self.owner == address && matches!(self.expiry, SubscriptionExpiryStatus::Owned(_))
    }

    pub fn is_owned(&self) -> bool {
        matches!(self.expiry, SubscriptionExpiryStatus::Owned(_))
    }

    pub fn is_expired(&self) -> bool {
        matches!(self.expiry, SubscriptionExpiryStatus::Expired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, SchemaType)]
pub struct GetRoyaltiesParams {
    pub token_id: TokenIdVec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, SchemaType)]
pub struct Royalty {
    pub beneficiary: AccountAddress,
    pub percentage: Percentage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, SchemaType)]
pub struct GetRoyaltiesResponse {
    pub royalties: Vec<Royalty>,
}
