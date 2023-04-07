use super::*;

pub type ContractResult<A> = Result<A, ContractError>;

/// Contract token ID type.
/// To save bytes we use a small token ID type, but is limited to be represented
/// by a `u8`.
pub type ContractTokenId = TokenIdVec;

/// Wrapping the custom errors in a type with CIS1 errors.
pub type ContractError = Cis2Error<CustomContractError>;

pub type UpdatePriceParameter = UpdatePriceParams<ContractTokenId>;

pub type MintParams = MintData<ContractTokenId>;

pub type TransferParameter = TransferParams<ContractTokenId, TokenAmountU64>;

/// Parameter type for the CIS-1 function `balanceOf` specialized to the subset
/// of TokenIDs used by this contract.
pub type ContractBalanceOfQueryParams = BalanceOfQueryParams<ContractTokenId>;
