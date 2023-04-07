use super::*;

/// The custom errors the contract can produce.
#[derive(Serialize, Debug, PartialEq, Eq, Reject, SchemaType)]
pub enum CustomContractError {
    /// Failed parsing the parameter (Error code: -1).
    #[from(ParseError)]
    ParseParams,
    /// Failed logging: Log is full (Error code: -2).
    LogFull,
    /// Failed logging: Log is malformed (Error code: -3).
    LogMalformed,
    /// Failing to mint new tokens because one of the token IDs already exists
    /// in this contract (Error code: -4).
    TokenIdAlreadyExists,
    /// Token is not listed for sale (Error code: -5).
    TokenNotListedForSale,
    /// Address Not Found (Error code: -6).
    AddressNotFound,
    /// Malformed asset hash (Error code: -7).
    InvalidHash,
    /// Unknown token (Error code: -8).
    UnknownToken,
    /// This current operation does not exist (Error code: -9)
    OperationDoesNotExist,
    /// Only account addresses can buy tokens (Error code: -10).
    OnlyAccountAddress,
    /// Only the contract owner can list tokens (Error code: -11).
    OnlyOwner,
    /// Only the contract owner has access (Error code: -12).
    OnlyContractOwner,
    /// Royalty validation error (Error code: -13).
    InvalidRoyalty,
    /// Token is already listed for sale (Error code: -14).
    TokenAlreadyListedForSale,
    // Raised if bid is lower than highest amount (Error code: -15)
    BidTooLow,
    // Attempt to update finished auction (Error code: -16)
    AuctionFinished,
    // Raised if bid is placed after auction has been finalized
    // (Error code: -17)
    AuctionFinalized,
    // Raised if there is a mistake in the bid map that keeps track of all
    // accounts' bids (Error code: -18)
    BidMapError,
    // Raised if there is an attempt to finalize the auction before its expiry
    // (Error code: -19)
    AuctionStillActive,
    // Raised if auction has been canceled (Error code: -20)
    AuctionCanceled,
    // Raised if auction contract is already authorised to perforn transactions
    // on NFT contract(Error code: -21)
    AlreadyAuthorized,
    /// Request from this address is already in progress (Error code: -22)
    RequestInProgress,
    /// No requests from this address in progress (Error code: -23)
    NoRequestInProgress,
    /// This function must only be called by a contract (Error code: -24)
    ContractOnly,
    /// Attempt to call function on an uninitialized contract (Error code: -25)
    NotInitialized,
    /// Contract was already initialized (Error code: -26)
    AlreadyInitialized,
    /// Only storage root can call this function (Error code: -27)
    RootOnly,
    /// Contract does not have spare instances to store extra data (Error code: -28)
    InsufficientInstances,
    /// Invalid Fields (Error code: -29)
    InvalidFields,
    /// Not Found (Error code: -30)
    NotFound,
    /// Duration is either too far in the future or in the past (Error code: -31)
    InvalidDuration,
    /// Operation not permitted (Error code: -32)
    OperationNotPermitted,
    /// Failed to invoke a contract (Error code: -33).
    InvokeContractError,
    /// Failed to invoke a transfer (Error code: -34).
    InvokeTransferError,
    /// Already exists (Error code: -35)
    AlreadyExists,
    /// Unauthorized (Error code: -36)
    Unauthorized,
    /// Incompatible contract (Error code: -37)
    Incompatible,
    /// Invalid domain format (Error code: -38)
    InvalidDomainFormat,
    /// Unsupported (Error code: -39)
    Unsupported,
    /// Owner is not allowed to perform this action (Error code: -40)
    OwnerForbidden,
    /// Owner is not allowed to perform this action (Error code: -41)
    AuctionNotStarted,
}

/// Mapping the logging errors to CustomContractError.
impl From<LogError> for CustomContractError {
    fn from(le: LogError) -> Self {
        match le {
            LogError::Full => Self::LogFull,
            LogError::Malformed => Self::LogMalformed,
        }
    }
}

/// Mapping errors related to contract invocations to CustomContractError.
impl<T> From<CallContractError<T>> for CustomContractError {
    fn from(_cce: CallContractError<T>) -> Self {
        Self::InvokeContractError
    }
}

/// Mapping CustomContractError to ContractError
impl From<CustomContractError> for ContractError {
    fn from(c: CustomContractError) -> Self {
        Cis2Error::Custom(c)
    }
}

/// Mapping errors related to contract invocations to CustomContractError.
impl From<TransferError> for CustomContractError {
    fn from(_te: TransferError) -> Self {
        Self::InvokeTransferError
    }
}

#[derive(Debug)]
pub enum ContractReadError<R> {
    Call(CallContractError<R>),
    Compatibility,
    Parse,
}
