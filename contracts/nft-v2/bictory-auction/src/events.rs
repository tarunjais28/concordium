use super::*;

/// An untagged event of biding.
#[derive(Debug, Serialize, SchemaType)]
pub struct BidingEvent {
    /// Account who has bidden.
    pub token_id: ContractTokenId,
    /// Biding Amount.
    pub bid: Amount,
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent {
    /// Biding
    Biding(BidingEvent),
    /// Finalize Auction
    Finalize(ContractTokenId),
    /// Cancel Auction
    Cancel(ContractTokenId),
}

impl Serial for CustomEvent {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CustomEvent::Biding(event) => {
                out.write_u8(BIDING_TAG)?;
                event.serial(out)
            }
            CustomEvent::Finalize(event) => {
                out.write_u8(FINALIZE_TAG)?;
                event.serial(out)
            }
            CustomEvent::Cancel(event) => {
                out.write_u8(CANCEL_TAG)?;
                event.serial(out)
            }
        }
    }
}

impl Deserial for CustomEvent {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let tag = source.read_u8()?;
        match tag {
            BIDING_TAG => BidingEvent::deserial(source).map(CustomEvent::Biding),
            FINALIZE_TAG => ContractTokenId::deserial(source).map(CustomEvent::Finalize),
            CANCEL_TAG => ContractTokenId::deserial(source).map(CustomEvent::Cancel),
            _ => Err(ParseError::default()),
        }
    }
}
