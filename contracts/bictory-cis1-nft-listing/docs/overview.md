# Listing

Listing contract for CIS-1 tokens with CIS Royalty extension.

## Compatibility

Listed tokens must implement following standards or specifications:

* [CIS-1](https://proposals.concordium.software/CIS/cis-1.html)
* [CIS Royalty extension](../../../docs/specs/cis_royalty.md)


## Types

```
TokenId ::= (size: u8) (buffer: u8 * size)
```

```
AccountAddress ::= (address: u8 * 32)
```

```
ContractAddress ::= (index: u64 as LE) (subindex: u64 as LE)
```

```
Amount ::= (micro_ccd: u64 as LE)
```

```
ListingInfo ::= (price: Amount)
```

```
Token ::= (contract: ContractAddress) (id: TokenId)
```

```
Percentage ::= (micro_percent: u64 as LE)
```

```
Royalty ::= (beneficiary: AccountAddress) (percentage: Percentage)
```


## Events

```
ListEvent ::= (tag: u8 = 247) (token: Token) (owner: AccountAddress) (price: Amount)
```

```
UnlistEvent ::= (tag: u8 = 249) (token: Token) (owner: AccountAddress)
```

```
BuyEvent ::= (tag: u8 = 248) (token: Token) (seller: AccountAddress) (buyer: AccountAddress) (price: Amount) (seller_share: Amount) (royalty_len: u32) (royalties: Royalty * royalty_len)
```


## Write functions

### Function `list`

Full name: `BictoryNftListing.list`

List transfered token for sale.

To list token for sale, it must be trasnfered via CIS-1 `transfer` function to this `BictoryNftListing` contract
instance with this function as a callback. Token price is specified via additional data field of CIS-1 `transfer`
function. See [CIS-1 transfer documentation](https://proposals.concordium.software/CIS/cis-1.html#transfer).


#### AdditionalData in binary

```
AdditionalData ::= (size: u16 = 8; total data size in bytes) (price: Amount)
```

#### Events

Produces `ListEvent` per each listed token.

#### Errors

* `-13 InvalidRoyalty`
  * Token royalty exceeds 100%;
* `-24 ContractOnly`
  * Transaction sent by an account.
* `-33 InvokeContractError`
  * NFT contract logic rejected.
* `-37 Incompatible`
  * NFT contract doesn't implement CIS Royalty Extension specification.
* `-39 Unsupported`
  * Transfer amount is over 1 token;
  * Token owner is a contract.
* `-2147483645 LogError::Full`
* `-2147483646 ParseError`
  * Invalid CIS-1 callback parameter;
  * Invalid AdditionalData format.


### Function `unlist`

Full name: `BictoryNftListing.unlist`

Unlist token from sale and return it to original owner.

#### Parameters in binary

```
Parameter ::= (token: Token)
```

#### Events

Produces `UnlistEvent`.

#### Errors

* `-5 TokenNotListedForSale`
* `-33 InvokeContractError`
  * NFT contract logic rejected.
* `-36 Unauthorized`
  * Attempt to unlist token as a non-owner account.
* `-37 Incompatible`
  * NFT contract doesn't implement CIS-1.
* `-2147483645 LogError::Full`
* `-2147483646 ParseError`


### Function `buy`

Full name: `BictoryNftListing.buy`

Buy listed NFT. Transfer all royalties and payment to the previous owner. Transfer NFT token to buyer.

#### Parameters in binary

```
Parameter ::= (token: Token)
```

#### Events

Produces `BuyEvent`.

#### Errors

* `-5 TokenNotListedForSale`
* `-10 OnlyAccountAddress`
* `-37 Incompatible`
  * NFT contract doesn't implement CIS-1.
* `-2147483634 MissingAccount`
* `-2147483635 AmountTooLarge`
* `-2147483645 LogError::Full`
* `-2147483646 ParseError`
