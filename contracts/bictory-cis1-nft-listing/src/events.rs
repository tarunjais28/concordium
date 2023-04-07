use commons_v1::{ContractTokenId, Percentage, Royalty, BUY_TAG, LISTING_TAG, UNLISTING_TAG};
use concordium_std::*;

/// Token list event data.
#[derive(Debug, Serial)]
pub struct ListEvent<'a> {
    /// Token contract address.
    pub contract: &'a ContractAddress,
    /// Token identifier.
    pub id: &'a ContractTokenId,
    /// Address of the token owner.
    pub owner: &'a AccountAddress,
    /// Listed price.
    pub price: Amount,
    /// Platform fee.
    pub platform_fee: Percentage,
}

/// Token unlist event data.
#[derive(Debug, Serial)]
pub struct UnlistEvent<'a> {
    /// Token contract address.
    pub contract: &'a ContractAddress,
    /// Token identifier.
    pub id: &'a ContractTokenId,
    /// Address of the token owner.
    pub owner: &'a AccountAddress,
}

/// Token buy event data.
#[derive(Debug, Serial)]
pub struct BuyEvent<'a> {
    /// Token contract address.
    pub contract: &'a ContractAddress,
    /// Token identifier.
    pub id: &'a ContractTokenId,
    /// Previous token owner.
    pub seller: &'a AccountAddress,
    /// New token owner.
    pub buyer: &'a AccountAddress,
    /// Price.
    pub price: Amount,
    /// Seller share.
    pub seller_share: Amount,
    /// Royalties.
    pub royalties: &'a Vec<Royalty>,
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum ListingEvent<'a> {
    /// List NFT
    List(ListEvent<'a>),
    /// Unlisting NFT
    Unlist(UnlistEvent<'a>),
    /// Buying NFT
    Buy(BuyEvent<'a>),
}

impl<'a> ListingEvent<'a> {
    pub fn list(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        owner: &'a AccountAddress,
        price: Amount,
        platform_fee: Percentage,
    ) -> Self {
        Self::List(ListEvent {
            contract,
            id,
            owner,
            price,
            platform_fee,
        })
    }

    pub fn unlist(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        owner: &'a AccountAddress,
    ) -> Self {
        Self::Unlist(UnlistEvent {
            contract,
            id,
            owner,
        })
    }

    pub fn buy(
        contract: &'a ContractAddress,
        id: &'a ContractTokenId,
        seller: &'a AccountAddress,
        buyer: &'a AccountAddress,
        price: Amount,
        seller_share: Amount,
        royalties: &'a Vec<Royalty>,
    ) -> Self {
        Self::Buy(BuyEvent {
            contract,
            id,
            seller,
            buyer,
            price,
            seller_share,
            royalties,
        })
    }
}

impl<'a> Serial for ListingEvent<'a> {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            ListingEvent::Unlist(event) => {
                out.write_u8(UNLISTING_TAG)?;
                event.serial(out)
            }
            ListingEvent::Buy(event) => {
                out.write_u8(BUY_TAG)?;
                event.serial(out)
            }
            ListingEvent::List(event) => {
                out.write_u8(LISTING_TAG)?;
                event.serial(out)
            }
        }
    }
}
