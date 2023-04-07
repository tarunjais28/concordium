use super::*;

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent {
    /// Updating Address
    UpdateAddress(AccountAddress),
}

impl Serial for CustomEvent {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CustomEvent::UpdateAddress(event) => {
                out.write_u8(UPDATE_ACCOUNT_TAG)?;
                event.serial(out)
            }
        }
    }
}

impl Deserial for CustomEvent {
    fn deserial<R: Read>(source: &mut R) -> ParseResult<Self> {
        let tag = source.read_u8()?;
        match tag {
            UPDATE_ACCOUNT_TAG => AccountAddress::deserial(source).map(CustomEvent::UpdateAddress),
            _ => Err(ParseError::default()),
        }
    }
}
