# Auction

Auction contract for CIS-1 tokens with CIS Royalty extension.

## Compatibility

Auctioned tokens must implement following standards or specifications:

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
Token ::= (contract: ContractAddress) (id: TokenId)
```

```
Percentage ::= (micro_percent: u64 as LE)
```

```
Royalty ::= (beneficiary: AccountAddress) (percentage: Percentage)
```

```
MaybeStartTime ::= (tag: u8 = 0; Now)
                 | (tag: u8 = 1; At) (start: Timestamp)

Finalization ::= (tag: u8 = 0; Duration)   (duration: Duration)
               | (tag: u8 = 1; BidTimeout) (duration: Duration)

BidIncrement ::= (tag: u8 = 0; Flat)       (amount: Amount)
               | (tag: u8 = 1; Percentage) (percentage: Percentage)

MaybeBuyout ::= (tag: u8 = 0; Disallow)
              | (tag: u8 = 1; Allow) (amount: Amount)

LotInfo ::= (start: MaybeStartTime) (finalization: Finalization) (reserve: Amount) (increment: BidIncrement) (buyout: MaybeBuyout)
```



## Events

```
AuctionEvent ::= (tag: u8 = 247) (token: Token) (owner: AccountAddress) (conditions: LotInfo)
```

```
BidEvent ::= (tag: u8 = 244) (token: Token) (bidder: AccountAddress) (amount: Amount)
```

```
CancelEvent ::= (tag: u8 = 242) (token: Token) (owner: AccountAddress)
```

```
AbortEvent ::= (tag: u8 = 233) (token: Token) (owner: AccountAddress) (bidder: AccountAddress) (amount: Amount)
```

```
FinalizeEvent ::= (tag: u8 = 243) (token: Token) (seller: AccountAddress) (winner: AccountAddress) (price: Amount) (seller_share: Amount) (royalty_length: u32 as LE) (royalties: Royalty * royalty_length)
```


## Write functions

### Function `auction`

Full name: `BictoryNftAuction.auction`

Auction transfered token.

To auction token, it must be trasnfered via CIS-1 `transfer` function to this `BictoryNftAuction` contract instance
with this function as a callback. Auction conditions are specified via additional data field of CIS-1 `transfer`
function. See [CIS-1 transfer documentation](https://proposals.concordium.software/CIS/cis-1.html#transfer).

Auction condition options:

* `start` - auction start time:
    * `Now` - start auction immediately;
    * `At` - start auction at given time;
* `finalization` - auction finalization policy:
    * `Duration` - auction is consedered complete after given duration from start time;
    * `BidTimeout` - auction is consedered complete after given duration from previous bid;
* `reserve` - minimum first bid;
* `increment` - minimal bid increment policy:
    * `Flat` - next bid must be at least previous bid + given amount;
    * `Percentage` - next bid must be at least previous bid * (100% + given percentage);
* `buyout` - auction buyout policy:
    * `Allow` - bids with amount no less than `buyout` complete auction instantly;
    * `Disallow` - buyouts are not allowed.

NOTE: Zero `increment` allows next EQUAL bid to be considered highest bid!

#### AdditionalData in binary

```
AdditionalData ::= (size: u16 as LE; total data size in bytes) (info: LotInfo)
```

#### Events

Produces `AuctionEvent` per each auctioned token.

#### Errors

* `-13 InvalidRoyalty`
  * Token royalty exceeds 100%;
  * Individual royalty entry count is 10 or higher.
* `-14 TokenAlreadyListedForSale`
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


### Function `bid`

Full name: `BictoryNftAuction.bid`

Bid on given lot. Previous bid gets refunded on this function call.

#### Parameters in binary

```
Parameter ::= (token: Token)
```

#### Events

Produces `BidEvent`.

#### Errors

* `-8 UnknownToken`
* `-10 OnlyAccountAddress`
* `-15 BidTooLow`
* `-16 AuctionFinished`
* `-40 OwnerForbidden`
* `-41 AuctionNotStarted`
* `-2147483645 LogError::Full`
* `-2147483646 ParseError`


### Function `finalize`

Full name: `BictoryNftAuction.finalize`

Finalize given lot. Transfer all royalties to corresponding addresses and highest bid amount to the previous owner.
Transfer NFT token to winning bidder.

#### Parameters in binary

```
Parameter ::= (token: Token)
```

#### Events

Produces following events based on outcome:

* `FinalizeEvent` - Successful auction finalization, token was transferred to new owner;
* `CancelEvent` - No bids on auction finalization, token was returned to the previous owner;
* `AbortEvent` - Token transfer error, bids were refunded. Token may be recoverable by `recover` function.

#### Errors

* `-8 UnknownToken`
* `-19 AuctionStillActive`
* `-2147483645 LogError::Full`
* `-2147483646 ParseError`


### Function `cancel`

Full name: `BictoryNftAuction.cancel`

Refund last bid and withdraw token from auction, returning it to original owner. This function can only be called
before auction completion and only by token owner.

#### Parameters in binary

```
Parameter ::= (token: Token)
```

#### Events

Produces `CancelEvent`.

#### Errors

* `-8 UnknownToken`
* `-10 OnlyAccountAddress`
* `-16 AuctionFinished`
* `-36 Unauthorized`
  * Attempt to cancel auction as a non-owner account.
* `-2147483645 LogError::Full`
* `-2147483646 ParseError`


### Function `recover`

Full name: `BictoryNftAuction.recover`

Attemt to recover the token that couldn't be transferred on auction finalization earlier.

#### Parameters in binary

```
Parameter ::= (token: Token)
```

#### Errors

* `-8 UnknownToken`
* `-33 InvokeContractError`
  * NFT contract logic rejected.
* `-36 Unauthorized`
  * Auction for this token was not aborted.
* `-37 Incompatible`
  * NFT contract doesn't implement CIS-1.
* `-2147483646 ParseError`
