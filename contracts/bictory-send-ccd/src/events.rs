use super::*;

/// An untagged event of Send CCD.
#[derive(Debug, Serialize, SchemaType)]
pub struct SendCCDEvent {
    /// Account to whom money will be transferred.
    pub account: AccountAddress,
    /// Amount.
    pub amount: Amount,
}

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent {
    /// Sending CCD
    Send(SendCCDEvent),
}

impl Serial for CustomEvent {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CustomEvent::Send(event) => {
                out.write_u8(SEND_CCD_TAG)?;
                event.serial(out)
            }
        }
    }
}

impl Deserial for CustomEvent {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let tag = source.read_u8()?;
        match tag {
            SEND_CCD_TAG => SendCCDEvent::deserial(source).map(CustomEvent::Send),
            _ => Err(ParseError::default()),
        }
    }
}
