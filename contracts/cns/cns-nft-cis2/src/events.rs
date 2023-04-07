use commons::{ContractTokenId, LEND_TAG};
use concordium_std::*;

/// Tagged Custom event to be serialized for the event log.
#[derive(Debug)]
pub enum CustomEvent {
    /// Increasing expiry duration of domain
    Lend {
        token: ContractTokenId,
        expiry: Timestamp,
    },
}

impl Serial for CustomEvent {
    fn serial<W: Write>(&self, out: &mut W) -> Result<(), W::Err> {
        match self {
            CustomEvent::Lend { token, expiry } => {
                out.write_u8(LEND_TAG)?;
                token.serial(out)?;
                expiry.serial(out)
            }
        }
    }
}
