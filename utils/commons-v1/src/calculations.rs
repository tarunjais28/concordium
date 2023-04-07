use super::*;

// convert the royalty percentage and amount to pay into a payout
fn royalty_to_payout(price: Amount, royalty: u64) -> Amount {
    let ttl_per = Amount::from_ccd(100);
    Amount::from_micro_ccd(price.micro_ccd * royalty / ttl_per.micro_ccd)
}

pub fn calc_shares(
    price: Amount,
    creator_royalty: u64,
    minter_royalty: u64,
    bictory_royalty: u64,
) -> Shares {
    Shares {
        creator: royalty_to_payout(price, creator_royalty),
        minter: royalty_to_payout(price, minter_royalty),
        owner: price,
        bictory: royalty_to_payout(price, bictory_royalty),
    }
}

#[concordium_cfg_test]
mod tests {
    use super::*;

    #[concordium_test]
    fn test_shares() {
        let expected_shares = Shares {
            creator: Amount::from_ccd(5),
            minter: Amount::from_ccd(2),
            owner: Amount::from_ccd(91),
            bictory: Amount::from_ccd(2),
        };

        let mut actual_shares = calc_shares(
            Amount::from_ccd(100),
            Amount::from_ccd(5).micro_ccd,
            Amount::from_ccd(2).micro_ccd,
            Amount::from_ccd(2).micro_ccd,
        );
        actual_shares.adjust_owner_share();

        claim_eq!(expected_shares, actual_shares);
    }
}
