use super::*;

/// The custom errors the contract can produce.
#[derive(Serialize, Debug, PartialEq, Eq, Reject)]
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
    /// The amount is insufficient to buy the token (Error code: -9).
    InsufficientAmount,
    /// Only account addresses can buy tokens (Error code: -10).
    OnlyAccountAddress,
    /// Only the contract owner can list tokens (Error code: -11).
    OnlyOwner,
    /// Only the contract owner has access (Error code: -12).
    OnlyContractOwner,
    /// Source and Target royalties are different (Error code: -13).
    MismatchInRoyalties,
    /// Token is already listed for sale (Error code: -14).
    TokenAlreadyListedForSale,
    // Raised if bid is lower than highest amount (Error code: -15)
    BidTooLow,
    // Raised if bid is placed after auction expiry time (Error code: -16)
    BidsOverWaitingForAuctionFinalization,
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

/// Mapping CustomContractError to ContractError
impl From<CustomContractError> for ContractError {
    fn from(c: CustomContractError) -> Self {
        Cis1Error::Custom(c)
    }
}
