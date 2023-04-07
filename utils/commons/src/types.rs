use super::*;

pub type ContractResult<A> = Result<A, ContractError>;

/// Contract token ID type.
/// To save bytes we use a small token ID type, but is limited to be represented
/// by a `u8`.
pub type ContractTokenId = TokenIdVec;

/// Contract token amount type.
pub type ContractTokenAmount = TokenAmountU64;

/// Wrapping the custom errors in a type with CIS1 errors.
pub type ContractError = Cis2Error<CustomContractError>;

pub type UpdatePriceParameter = UpdatePriceParams<ContractTokenId>;

pub type MintParams = MintData<ContractTokenId>;

pub type TransferParameter = TransferParams<ContractTokenId, ContractTokenAmount>;

/// Parameter type for the CIS-2 function `balanceOf` specialized to the subset
/// of TokenIDs used by this contract.
pub type ContractBalanceOfQueryParams = BalanceOfQueryParams<ContractTokenId>;

/// Response type for the CIS-2 function `balanceOf` specialized to the subset
/// of TokenAmounts used by this contract.
pub type ContractBalanceOfQueryResponse = BalanceOfQueryResponse<ContractTokenAmount>;
