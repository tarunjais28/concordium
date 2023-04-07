use super::*;

/// An untagged event of tokens being Unlisted.
#[derive(Debug, Serialize, SchemaType)]
pub struct UpdatePriceEvent<T: IsTokenId> {
    /// The ID of the token whose price is being updated.
    pub token_id: T,
    /// The owner of the tokens being burned.
    pub owner: Address,
    /// Previous price of Token.
    pub from: Amount,
    /// Updated price of Token.
    pub to: Amount,
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent<T: IsTokenId> {
    /// Updating price of NFT
    UpdatePrice(UpdatePriceEvent<T>),
}

impl<T: IsTokenId> Serial for CustomEvent<T> {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CustomEvent::UpdatePrice(event) => {
                out.write_u8(UPDATE_PRICE_TAG)?;
                event.serial(out)
            }
        }
    }
}

impl<T: IsTokenId> Deserial for CustomEvent<T> {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let tag = source.read_u8()?;
        match tag {
            UPDATE_PRICE_TAG => {
                UpdatePriceEvent::<T>::deserial(source).map(CustomEvent::UpdatePrice)
            }
            _ => Err(ParseError::default()),
        }
    }
}
