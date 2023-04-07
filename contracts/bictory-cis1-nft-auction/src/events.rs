use crate::external::LotInfo;
use commons_v1::{
    ContractTokenId, Royalty, ABORT_TAG, BIDING_TAG, CANCEL_TAG, FINALIZE_TAG, LISTING_TAG,
};
use concordium_std::*;

/// NFT auction event data.
#[derive(Debug, Serial)]
pub struct AuctionEvent<'a> {
    /// NFT contract address.
    pub contract: &'a ContractAddress,
    /// NFT token identifier.
    pub id: &'a ContractTokenId,
    /// Address of the token owner.
    pub owner: &'a AccountAddress,
    /// AuctionConditions.
    pub conditions: &'a LotInfo,
}

/// Bid event data.
#[derive(Debug, Serial)]
pub struct BidEvent<'a> {
    /// NFT contract address.
    pub contract: &'a ContractAddress,
    /// NFT token identifier.
    pub id: &'a ContractTokenId,
    /// Bidder account address.
    pub bidder: &'a AccountAddress,
    /// Bid amount.
    pub amount: Amount,
}

/// Cancel auction event data.
#[derive(Debug, Serial)]
pub struct CancelEvent<'a> {
    /// NFT contract address.
    pub contract: &'a ContractAddress,
    /// NFT token identifier.
    pub id: &'a ContractTokenId,
    /// Address of the token owner.
    pub owner: &'a AccountAddress,
}

/// Abort auction event data.
#[derive(Debug, Serial)]
pub struct AbortEvent<'a> {
    /// NFT contract address.
    pub contract: &'a ContractAddress,
    /// NFT token identifier.
    pub id: &'a ContractTokenId,
    /// Address of the token owner.
    pub owner: &'a AccountAddress,
    /// Bidder account address.
    pub bidder: &'a AccountAddress,
    /// Bid amount.
    pub amount: Amount,
}

/// Auction finalization event data.
#[derive(Debug, Serial)]
pub struct FinalizeEvent<'a> {
    /// NFT contract address.
    pub contract: &'a ContractAddress,
    /// NFT token identifier.
    pub id: &'a ContractTokenId,
    /// Address of the previous token owner.
    pub seller: &'a AccountAddress,
    /// Address of the auction winner.
    pub winner: &'a AccountAddress,
    /// Winning auction bid.
    pub price: Amount,
    /// Seller share after deducting royalties.
    pub seller_share: Amount,
    /// Royalties.
    pub royalties: &'a Vec<Royalty>,
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum AuctionEvents<'a> {
    Auction(AuctionEvent<'a>),
    Bid(BidEvent<'a>),
    Cancel(CancelEvent<'a>),
    Abort(AbortEvent<'a>),
    Finalize(FinalizeEvent<'a>),
}

impl<'a> AuctionEvents<'a> {
    pub fn auction(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        owner: &'a AccountAddress,
        conditions: &'a LotInfo,
    ) -> Self {
        Self::Auction(AuctionEvent {
            contract,
            id,
            owner,
            conditions,
        })
    }

    pub fn bid(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        bidder: &'a AccountAddress,
        amount: Amount,
    ) -> Self {
        Self::Bid(BidEvent {
            contract,
            id,
            bidder,
            amount,
        })
    }

    pub fn cancel(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        owner: &'a AccountAddress,
    ) -> Self {
        Self::Cancel(CancelEvent {
            contract,
            id,
            owner,
        })
    }

    pub fn abort(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        owner: &'a AccountAddress,
        bidder: &'a AccountAddress,
        amount: Amount,
    ) -> Self {
        Self::Abort(AbortEvent {
            contract,
            id,
            owner,
            bidder,
            amount,
        })
    }

    pub fn finalize(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        seller: &'a AccountAddress,
        winner: &'a AccountAddress,
        price: Amount,
        seller_share: Amount,
        royalties: &'a Vec<Royalty>,
    ) -> Self {
        Self::Finalize(FinalizeEvent {
            contract,
            id,
            seller,
            winner,
            price,
            seller_share,
            royalties,
        })
    }
}

impl<'a> Serial for AuctionEvents<'a> {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            AuctionEvents::Auction(event) => {
                out.write_u8(LISTING_TAG)?;
                event.serial(out)
            }
            AuctionEvents::Bid(event) => {
                out.write_u8(BIDING_TAG)?;
                event.serial(out)
            }
            AuctionEvents::Cancel(event) => {
                out.write_u8(CANCEL_TAG)?;
                event.serial(out)
            }
            AuctionEvents::Abort(event) => {
                out.write_u8(ABORT_TAG)?;
                event.serial(out)
            }
            AuctionEvents::Finalize(event) => {
                out.write_u8(FINALIZE_TAG)?;
                event.serial(out)
            }
        }
    }
}
