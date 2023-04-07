# CNS NFT contract

CNS NFT contract is a CIS-1 NFT contract for CNS that features token expiry. It's responsible for keeping ownership
data for CNS. The address that ownes the token with id equal to namehash of the domain name is allowed to operate on
the corresponding domain name via CNS contract. For namehash algorithm description see either ENS or CNS documentation.

Token data is stored in a storage contract, so existing tokens can be used with other compatible and authorized NFT
contracts that are allowed to implement different token standards, like CIS-2.

Creating new tokens and extending the token subscription must happen via the CNS contract.

## Specifications

* [CIS-1](https://proposals.concordium.software/CIS/cis-1.html)
* [CIS Royalty](../../../../docs/specs/cis_royalty.md)

### CIS-1 implementation notes

1. Token data is stored separately, inside storage contract. This allows to update contract logic without loosing the token data.
2. Unlike token data, address operators are stored locally per each NFT contract.


## Types

```
TokenId ::= (size: u8) (buffer: u8 * size)
```

```
Address ::= (tag: u8 = 0) (address: AccountAddress)
          | (tag: u8 = 1) (address: ContractAddress)
```

```
AccountAddress ::= (address: u8 * 32)
```

```
ContractAddress ::= (index: u8 * 8) (subindex: u8 * 8)
```

```
Timestamp ::= (milliseconds: u64 as LE)
```

```
Percentage ::= (micro_percent: u64 as LE)
```


## Write functions

### Function `burn`

Receive name: `BictoryCnsNft.burn`

Anyone is allowed to call this function.

Can be called if token ownership and grace period have expired to clear the token data.

#### Parameters in binary

```
Parameter ::= (token_id: TokenId)
```

### Events

* `CIS-1 BurnEvent`


## Read functions

### Function `getTokenExpiry`

Receive name: `BictoryCnsNft.getTokenExpiry`

Get token expiry info.

#### Parameters in binary

```
Parameter ::= (token_id: TokenId)
```

#### Return value

If result tag is `0`, token does not exist. Otherwise, following variants can be returned:

* `Owned` - Token is owned by given address and expires at `expiry`;
* `Grace` - Token was owned by given address, but ownership expired token entered grace period that ends at `expiry`;
* `Expired` - Token was owned by given address, but it expired and grace period ended. This token can be burnt;

```
SubscriptionExpiryStatus ::= (tag: u8 = 0; Owned) (expiry: Timestamp)
                           | (tag: u8 = 1; Grace) (expiry: Timestamp)
                           | (tag: u8 = 2; Expired)

TokenSubscriptionStatus ::= (owner: Address) (expiry: SubscriptionExpiryStatus)

Result ::= (tag: u8 = 0; Token doesn't exist)
         | (tag: u8 = 1) (status: TokenSubscriptionStatus)
```


### Function `getTokenInfo`

Receive name: `BictoryCnsNft.getTokenInfo`

Get token info.

#### Parameters in binary

```
Parameter ::= (token_id: TokenId)
```

#### Return value

If result tag is `0`, token does not exist. Otherwise, following variants can be returned:

```
TokenInfo ::= (domain: String) (royalty: Percentage)

Result ::= (tag: u8 = 0; Token doesn't exist)
         | (tag: u8 = 1) (info: TokenInfo)
```
