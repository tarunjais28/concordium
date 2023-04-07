use super::*;

pub type ContractResult<A> = Result<A, ContractError>;

pub type TransferParameter = TransferParams<ContractTokenId>;

/// Parameter type for the CIS-1 function `balanceOf` specialized to the subset
/// of TokenIDs used by this contract.
pub type ContractBalanceOfQueryParams = BalanceOfQueryParams<ContractTokenId>;

pub type MintingParameter = MintParams<ContractTokenId>;
