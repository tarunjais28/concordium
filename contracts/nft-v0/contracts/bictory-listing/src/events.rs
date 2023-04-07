use super::*;

/// An untagged event of tokens being Unlisted.
#[derive(Debug, Serialize, SchemaType)]
pub struct UnlistingEvent {
    /// Struct containing contract_address, nft_group_id and contract_token_id.
    pub token: Token,
    /// for_sale flag.
    pub for_sale: bool,
}

/// An untagged event of buy tokens from seller.
#[derive(Debug, Serialize, SchemaType)]
pub struct BuyEvent {
    /// The ID of the token being purchased.
    pub token: Token,
    /// The address owning these tokens before buying.
    pub seller: AccountAddress,
    /// The address to receive these tokens after the sell.
    pub buyer: AccountAddress,
    /// Seller's share.
    pub owner_share: Amount,
    /// Creayers's share.
    pub creator_share: Amount,
    /// for_sale flag.
    pub for_sale: bool,
}

/// An untagged event of tokens being Listed.
#[derive(Debug, Serialize, SchemaType)]
pub struct ListingEvent {
    /// for_sale flag.
    pub for_sale: bool,
    /// Struct containing Token, owner, creator, NFT's price, royalty.
    pub listing: ListParams,
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent {
    /// Unlisting NFT
    Unlisting(UnlistingEvent),
    /// Buying NFT
    Buy(BuyEvent),
    /// Listing NFT
    Listing(ListingEvent),
}

impl Serial for CustomEvent {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CustomEvent::Unlisting(event) => {
                out.write_u8(UNLISTING_TAG)?;
                event.serial(out)
            }
            CustomEvent::Buy(event) => {
                out.write_u8(BUY_TAG)?;
                event.serial(out)
            }
            CustomEvent::Listing(event) => {
                out.write_u8(LISTING_TAG)?;
                event.serial(out)
            }
        }
    }
}

impl Deserial for CustomEvent {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let tag = source.read_u8()?;
        match tag {
            UNLISTING_TAG => UnlistingEvent::deserial(source).map(CustomEvent::Unlisting),
            BUY_TAG => BuyEvent::deserial(source).map(CustomEvent::Buy),
            LISTING_TAG => ListingEvent::deserial(source).map(CustomEvent::Listing),
            _ => Err(ParseError::default()),
        }
    }
}
