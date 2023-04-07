use commons_v1::{Authority, CustomContractError, Percentage, Royalty, Token};
use concordium_cis1::*;
use concordium_std::*;

use crate::external::{BidIncrement, Finalization, LotInfo};

pub enum AuctionState {
    NotStarted,
    Active,
    Completed,
}

impl AuctionState {
    fn is_completed(&self) -> bool {
        matches!(self, AuctionState::Completed)
    }
}

#[derive(Debug, Clone, Serialize, SchemaType)]
struct Bid {
    pub timestamp: Timestamp,
    pub account: AccountAddress,
    pub amount: Amount,
}

#[derive(Serialize)]
pub enum TokenState {
    Lot(LotData),
    Grave(AccountAddress),
}

impl TokenState {
    fn lot(self) -> Option<LotData> {
        match self {
            Self::Lot(lot) => Some(lot),
            _ => None,
        }
    }

    fn lot_mut(&mut self) -> Option<&mut LotData> {
        match self {
            Self::Lot(ref mut lot) => Some(lot),
            _ => None,
        }
    }
}

#[derive(Serialize, SchemaType)]
pub struct LotData {
    /// Seller account address.
    owner: AccountAddress,
    /// Platform fee on auction initialization.
    platform_fee: Percentage,
    /// Minimum bid amount.
    reserve: Amount,
    /// Minimum bid increment.
    increment: BidIncrement,
    /// Buyout price. Buyout is not allowed by default.
    buyout: Option<Amount>,
    /// Auction start time. Immediate by default.
    start: Timestamp,
    /// Auction finalization policy.
    finalization: Finalization,
    /// Current highest bid.
    highest_bid: Option<Bid>,
    /// Token royalties that get transfered on auction finalization.
    royalties: Vec<Royalty>,
}

impl LotData {
    pub fn new(
        owner: AccountAddress,
        platform_fee: Percentage,
        lot_info: LotInfo,
        slot_time: Timestamp,
        royalties: Vec<Royalty>,
    ) -> Self {
        Self {
            owner,
            platform_fee,
            reserve: lot_info.reserve,
            increment: lot_info.increment,
            buyout: lot_info.buyout,
            start: lot_info.start.unwrap_or(slot_time),
            finalization: lot_info.finalization,
            highest_bid: None,
            royalties,
        }
    }

    /// Get lot state at given slot_time
    pub fn auction_state(&self, slot_time: Timestamp) -> AuctionState {
        if slot_time < self.start {
            AuctionState::NotStarted
        } else {
            // Check buyout
            if let (Some(buyout), Some(highest_bid)) = (self.buyout, self.highest_bid.as_ref()) {
                if highest_bid.amount >= buyout {
                    return AuctionState::Completed;
                }
            }
            // Check buyout
            match self.finalization {
                Finalization::Duration(duration) => {
                    if slot_time <= self.start.checked_add(duration).unwrap() {
                        AuctionState::Active
                    } else {
                        AuctionState::Completed
                    }
                }
                Finalization::BidTimeout(timeout) => {
                    if slot_time
                        <= self
                            .highest_bid
                            .as_ref()
                            .map(|bid| bid.timestamp)
                            .unwrap_or(self.start)
                            .checked_add(timeout)
                            .unwrap()
                    {
                        AuctionState::Active
                    } else {
                        AuctionState::Completed
                    }
                }
            }
        }
    }
}

/// Final auction bid. On cancel or overbid it must be refunded. On finalization funds must be transferred to seller after deducting all fees.
#[must_use]
pub struct LastBid {
    pub account: AccountAddress,
    pub amount: Amount,
}

impl From<Bid> for LastBid {
    fn from(bid: Bid) -> Self {
        Self {
            account: bid.account,
            amount: bid.amount,
        }
    }
}

/// Auction winner.
#[must_use]
pub enum AuctionResult {
    /// Highest bid
    Winner {
        previous_owner: AccountAddress,
        winning_bid: LastBid,
        royalties: Vec<Royalty>,
    },
    /// No bids were placed during the auction
    Refund(AccountAddress),
}

/// The contract state.
#[derive(Serial, DeserialWithState)]
#[concordium(state_parameter = "S")]
pub struct State<S: HasStateApi> {
    /// Authority module for administrative rights management.
    pub authority: Authority<S>,
    /// Platform royalty. Gets associated with auction on initialization till finalization.
    pub royalty: Percentage,
    /// Platform royalty receiver account.
    pub beneficiary: AccountAddress,
    /// Token data.
    pub tokens: StateMap<Token, TokenState, S>,
}

impl<S: HasStateApi> State<S> {
    /// Create a new state with no lots.
    pub fn new(
        state_builder: &mut StateBuilder<S>,
        beneficiary: AccountAddress,
        royalty: Percentage,
        origin: AccountAddress,
    ) -> Self {
        State {
            authority: Authority::new(state_builder, Address::Account(origin)),
            royalty,
            beneficiary,
            tokens: state_builder.new_map(),
        }
    }

    pub fn auction(
        &mut self,
        contract: ContractAddress,
        id: TokenIdVec,
        owner: AccountAddress,
        lot_info: LotInfo,
        slot_time: Timestamp,
        royalties: Vec<Royalty>,
    ) -> ReceiveResult<()> {
        let lot = LotData::new(owner, self.royalty, lot_info, slot_time, royalties);
        if let Some(previous_token_state) = self
            .tokens
            .insert(Token { contract, id }, TokenState::Lot(lot))
        {
            match previous_token_state {
                // Graves can be overwritten
                TokenState::Grave(_) => Ok(()),
                // Duplicate token auctioning is not allowed
                TokenState::Lot(_) => Err(CustomContractError::TokenAlreadyListedForSale.into()),
            }
        } else {
            Ok(())
        }
    }

    pub fn bid(
        &mut self,
        token: &Token,
        slot_time: Timestamp,
        bidder: AccountAddress,
        amount: Amount,
    ) -> Result<Option<LastBid>, CustomContractError> {
        let mut token = self
            .tokens
            .get_mut(token)
            .ok_or_else(|| CustomContractError::UnknownToken)?;
        let lot = token
            .get_mut()
            .lot_mut()
            .ok_or_else(|| CustomContractError::UnknownToken)?;

        match lot.auction_state(slot_time) {
            AuctionState::NotStarted => bail!(CustomContractError::AuctionNotStarted),
            AuctionState::Active => (),
            AuctionState::Completed => bail!(CustomContractError::AuctionFinished),
        }

        // Owner is not allowed to raise bids
        ensure_ne!(bidder, lot.owner, CustomContractError::OwnerForbidden);

        if let Some(bid) = &lot.highest_bid {
            // Check minimum increment
            match lot.increment {
                BidIncrement::Flat(minimum_increment) => {
                    ensure!(
                        amount >= bid.amount + minimum_increment,
                        CustomContractError::BidTooLow
                    )
                }
                BidIncrement::Percentage(increment) => {
                    ensure!(
                        Percentage::from_percent(100) + increment
                            <= Percentage::of_amount(amount, bid.amount),
                        CustomContractError::BidTooLow
                    )
                }
            }
        } else {
            // Check minimum bid
            ensure!(amount >= lot.reserve, CustomContractError::BidTooLow);
        }

        // Update the highest bid after all checks, return the previous bid that MUST be refunded
        Ok(lot
            .highest_bid
            .replace(Bid {
                timestamp: slot_time,
                account: bidder,
                amount,
            })
            .map(Into::into))
    }

    pub fn finalize(
        &mut self,
        token: &Token,
        slot_time: Timestamp,
    ) -> Result<AuctionResult, CustomContractError> {
        self.tokens
            .remove_and_get(token)
            .and_then(|token_state| token_state.lot())
            .ok_or_else(|| CustomContractError::UnknownToken)
            .and_then(|lot| {
                if lot.auction_state(slot_time).is_completed() {
                    let previous_owner = lot.owner;
                    let mut royalties = lot.royalties;
                    royalties.push(Royalty {
                        beneficiary: self.beneficiary,
                        percentage: lot.platform_fee,
                    });
                    let result = lot
                        .highest_bid
                        // Return the highest bid
                        .map(|bid| AuctionResult::Winner {
                            previous_owner,
                            winning_bid: bid.into(),
                            royalties,
                        })
                        // Or provide token refund info
                        .unwrap_or(AuctionResult::Refund(previous_owner));
                    Ok(result)
                } else {
                    // Finalizing non-completed auction is not allowed
                    Err(CustomContractError::AuctionStillActive)
                }
            })
    }

    pub fn cancel(
        &mut self,
        token: &Token,
        sender: &AccountAddress,
        slot_time: Timestamp,
    ) -> Result<(AccountAddress, Option<LastBid>), CustomContractError> {
        self.tokens
            .remove_and_get(token)
            .and_then(|token_state| token_state.lot())
            .ok_or_else(|| CustomContractError::UnknownToken)
            .and_then(|lot| {
                ensure_eq!(sender, &lot.owner, CustomContractError::Unauthorized);
                if lot.auction_state(slot_time).is_completed() {
                    // Cancelling finished auction is not allowed
                    Err(CustomContractError::AuctionFinished)
                } else {
                    // Return the last bid that MUST be refunded
                    Ok((lot.owner, lot.highest_bid.map(Into::into)))
                }
            })
    }

    /// Replace token data with a token grave that may be returned later.
    ///
    /// Must only ever be called after removing the lot.
    pub fn bury(&mut self, token: Token, owner: AccountAddress) {
        self.tokens.insert(token, TokenState::Grave(owner));
    }

    // Replace token data with a token grave that may be returned later
    pub fn recover(&mut self, token: &Token) -> Result<AccountAddress, CustomContractError> {
        // Check if grave is present for this token
        let owner = match *self
            .tokens
            .get(token)
            .ok_or_else(|| CustomContractError::UnknownToken)?
        {
            TokenState::Lot(_) => Err(CustomContractError::Unauthorized)?,
            TokenState::Grave(owner) => owner,
        };

        // Clean it up
        self.tokens.remove(&token);

        Ok(owner)
    }
}
