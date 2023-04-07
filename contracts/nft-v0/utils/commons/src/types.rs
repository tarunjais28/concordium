use super::*;

/// Contract token ID type.
/// To save bytes we use a small token ID type, but is limited to be represented
/// by a `u8`.
pub type ContractTokenId = TokenIdVec;

/// Wrapping the custom errors in a type with CIS1 errors.
pub type ContractError = Cis1Error<CustomContractError>;

pub type UpdatePriceParameter = UpdatePriceParams<ContractTokenId>;
