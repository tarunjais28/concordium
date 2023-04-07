use super::*;

// convert the royalty percentage and amount to pay into a payout
fn royalty_to_payout(price: Amount, royalty: u64) -> Amount {
    let ttl_per = Amount::from_ccd(100);
    Amount::from_micro_ccd(price.micro_ccd * royalty / ttl_per.micro_ccd)
}

pub fn calc_shares(price: Amount, bictory_royalty: u64) -> Shares {
    let royalty_to_creator = 100_000_000 - bictory_royalty;
    Shares {
        creator: royalty_to_payout(price, royalty_to_creator),
        bictory: royalty_to_payout(price, bictory_royalty),
    }
}

#[concordium_cfg_test]
mod tests {
    use super::*;

    #[concordium_test]
    fn test_shares() {
        let expected_shares = Shares {
            creator: Amount::from_ccd(95),
            bictory: Amount::from_ccd(5),
        };

        let actual_shares = calc_shares(Amount::from_ccd(100), Amount::from_ccd(5).micro_ccd);

        claim_eq!(expected_shares, actual_shares);
    }
}
