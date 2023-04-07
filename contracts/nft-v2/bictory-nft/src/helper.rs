use super::*;

/// Build a string from TOKEN_METADATA_BASE_URL appended with the token ID
/// encoded as hex.
pub fn build_token_metadata_url(token_id: &ContractTokenId) -> String {
    let mut token_metadata_url = String::from(TOKEN_METADATA_BASE_URL);
    push_token_id(&mut token_metadata_url, token_id);
    token_metadata_url.push('/');
    token_metadata_url
}

pub fn push_token_id(string: &mut String, token_id: &ContractTokenId) {
    for byte in &token_id.0 {
        string.push(bits_to_hex_char(byte >> 4));
        string.push(bits_to_hex_char(byte & 0xF));
    }
}

pub fn bits_to_hex_char(bits: u8) -> char {
    match bits & 0xF {
        0x0..=0x9 => (bits + b'0') as char,
        0xA..=0xF => (bits - 10 + b'A') as char,
        _ => unreachable!(),
    }
}

pub fn token_metadata_event(token_id: ContractTokenId) -> Cis1Event<ContractTokenId> {
    let token_metadata_url = build_token_metadata_url(&token_id);
    Cis1Event::TokenMetadata(TokenMetadataEvent {
        token_id,
        metadata_url: MetadataUrl {
            url: token_metadata_url,
            hash: None,
        },
    })
}

#[concordium_cfg_test]
mod tests {
    use super::*;

    #[concordium_test]
    fn token_id_formatting() {
        for x in 0x00u8..0xFF {
            let mut counter = x;
            let token_bytes = core::iter::repeat_with(|| {
                let res = counter;
                counter = counter.wrapping_add(0x55);
                res
            })
            .take(x as usize % 10 + 1)
            .collect();

            let token_id: ContractTokenId = TokenIdVec(token_bytes);

            let mut token_id_string = String::new();
            push_token_id(&mut token_id_string, &token_id);
            claim_eq!(token_id_string, token_id.to_string());
        }
    }
}
