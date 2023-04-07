use super::*;

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
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent {
    /// Unlisting NFT
    Unlisting(ListParams),
    /// Buying NFT
    Buy(BuyEvent),
    /// Listing NFT
    Listing(ListParams),
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
            UNLISTING_TAG => ListParams::deserial(source).map(CustomEvent::Unlisting),
            BUY_TAG => BuyEvent::deserial(source).map(CustomEvent::Buy),
            LISTING_TAG => ListParams::deserial(source).map(CustomEvent::Listing),
            _ => Err(ParseError::default()),
        }
    }
}
