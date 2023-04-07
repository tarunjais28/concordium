use super::*;

/// An untagged event of tokens being set for sale.
/// For a tagged version, use `CustomEvent`.
// Note: For the serialization to be derived according to the CIS1
// specification, the order of the fields cannot be changed.
#[derive(Debug, Serialize, SchemaType)]
pub struct SetForSaleEvent<T: IsTokenId> {
    /// The ID of the token being updated, (possibly an old token ID).
    pub token_id: T,
    /// The number of tokens being minted, this is allowed to be 0 as well.
    pub for_sale: bool,
    /// The owner of the minted token.
    pub owner: Address,
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent<T: IsTokenId> {
    /// Setting for_sale flag event.
    SetForSale(SetForSaleEvent<T>),
}

impl<T: IsTokenId> Serial for CustomEvent<T> {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CustomEvent::SetForSale(event) => {
                out.write_u8(SET_FOR_SALE_EVENT_TAG)?;
                event.serial(out)
            }
        }
    }
}

impl<T: IsTokenId> Deserial for CustomEvent<T> {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let tag = source.read_u8()?;
        match tag {
            SET_FOR_SALE_EVENT_TAG => {
                SetForSaleEvent::<T>::deserial(source).map(CustomEvent::SetForSale)
            }
            _ => Err(ParseError::default()),
        }
    }
}
