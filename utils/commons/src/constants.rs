/// Tag for the Custom For Sale event.
pub const SET_FOR_SALE_EVENT_TAG: u8 = u8::MAX - 5;

/// Tag for the Custom For Unlisting event.
pub const UNLISTING_TAG: u8 = u8::MAX - 6;

/// Tag for the Custom Buy event.
pub const BUY_TAG: u8 = u8::MAX - 7;

/// Tag for the Custom Listing event.
pub const LISTING_TAG: u8 = u8::MAX - 8;

/// Tag for the Custom Update Account event.
pub const UPDATE_ACCOUNT_TAG: u8 = u8::MAX - 9;

/// Tag for the Manage CNS Contract event.
pub const CNS_CONTRACT_TAG: u8 = u8::MAX - 10;

/// Tag for the Custom Biding event.
pub const BIDING_TAG: u8 = u8::MAX - 11;

/// Tag for the Custom Finalize Biding event.
pub const FINALIZE_TAG: u8 = u8::MAX - 12;

/// Tag for the Custom Cancel Biding event.
pub const CANCEL_TAG: u8 = u8::MAX - 13;

/// Tag for the Custom Send CCD event.
pub const SEND_CCD_TAG: u8 = u8::MAX - 14;

/// Tag for the Custom Update Price event.
pub const UPDATE_PRICE_TAG: u8 = u8::MAX - 15;

/// Tag for the Storage InstanceAccepted event.
pub const INSTANCE_ACCEPT_TAG: u8 = u8::MAX - 16;

/// Tag for the Storage InstanceConsumed event.
pub const INSTANCE_CONSUME_TAG: u8 = u8::MAX - 17;

/// Tag for the Lend event.
pub const LEND_TAG: u8 = u8::MAX - 18;

/// Tag for the Manage Admin event.
pub const ADMIN_TAG: u8 = u8::MAX - 19;

/// Tag for the Manage Maintaner event.
pub const MAINTAINER_TAG: u8 = u8::MAX - 20;

/// Tag for the SetYearlyDomainPrice event of CNS price oracle.
pub const SET_YEARLY_DOMAIN_PRICE_TAG: u8 = u8::MAX - 21;

/// Tag for the abort event.
pub const ABORT_TAG: u8 = u8::MAX - 22;

pub const OWNER: &str = "owner";
pub const CREATOR: &str = "creator";
pub const CREATOR_ROYALTY: &str = "creator_royalty";
pub const MINTER: &str = "minter";
pub const MINTER_ROYALTY: &str = "minter_royalty";
pub const PRICE: &str = "price";
pub const CID: &str = "cid";
pub const FOR_SALE: &str = "for_sale";

pub const TOKEN_FIELDS: [&str; 8] = [
    OWNER,
    CREATOR,
    CREATOR_ROYALTY,
    MINTER,
    MINTER_ROYALTY,
    PRICE,
    CID,
    FOR_SALE,
];
