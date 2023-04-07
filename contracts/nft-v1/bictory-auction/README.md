# Auction

Contract name: `BictoryAuction`


## Steps:
1. Initialize contract for auction with NFT's owner private key.
2. Call authorize function from NFT's owner private key.
3. Now anyone can call `bid` function by passing respective bidding amount.
4. Can be finalized by any private key after the end of expiry period. Only highest bidding will be transferred to NFT owner's account and rest will be returned to respective accounts.

Note: This contract can be cancelled anytime by calling `cancel` functionality. All the respective bidding amounts will automatically be returned to the respective bidder's account.

## Binary format

### Notation

Following notation is used to describe binary format:
* `::=` assigns name to a logical byte group
* `()` logical group of bytes
* `:` separates group name on the left and group type on the right
* `|` alternative group variant, usually used with tag
* `=` indicates allowed value this variant
* `u8, u16, u32, u64` number types, `u` for unsigned, `i` for signed, number indicates number size in bits
* `LE` number is expected in little endian format. Least significant byte comes first, most significant comes last
* `*` inside logical group indicates how many times it should be repeated
* `//` comment, does not affect data format

Example:

```
IpfsCid ::= (size: u32 as LE) (buffer: u8 * size)
```

New logical group is assigned a name `IpfsCid` to use as a type later. First group is named `size`, it expects a 4
byte number in little endian format. Next group is named `buffer`, it expects `size` bytes. So, if `size` is 16,
it must be followed by exactly 16 bytes.

Little endian transformation examples:

| Dec        | Byte array        | Hex        |
|------------|-------------------|------------|
| 123        | [123, 0, 0, 0]    | 0x7B000000 |
| 12345      | [57, 48, 0, 0]    | 0x39300000 |
| 12345678   | [78, 97, 188, 0]  | 0x4E61BC00 |
| 1234567890 | [210, 2, 150, 73] | 0x499602D2 |

### Types

```
Amount ::= (micro_ccd: u64 as LE)
```

```
Timestamp ::= (milliseconds: u64 as LE)
```

```
Token ::= (contract: ContractAddress) (id: TokenId)
```

```
Boolean ::= (bool: u8 = 0; false) 
          | (bool: u8 = 1; true)
```

```
Token ::= (contract: ContractAddress) (id: TokenId)
```

### Logs

```
BidingEvent ::= (tag: u8 = 244) (account: Token) (bid: Amount)
```

```
Finalize ::= (tag: u8 = 243) (token: Token)
```

```
Cancel ::= (tag: u8 = 242) (token: Token)
```

```
UpdateKind ::= (tag: byte = 0; Remove)
             | (tag: byte = 1; Add)

UpdateOperatorEvent ::= (tag: u8 = 252) (kind: UpdateKind) (owner: Address) (operator: Address)
```

## Function paramters

### Initialization

Full name: `init_BictoryAuction`

The new contract always be initialized during auction with `item` having contract_address and token_id with corresponding `expiry` time in GMT format. For example: `2022-03-02T06:35:00+00:00`.

#### Parameters as JSON

```
{
    "item": {
        "token": {
            "contract": {
                "index": <instance_index: number>,
                "subindex": <instance_subindex: number>
            },
            "id": <token_id: string with lowercase hex>
        },
        "expiry": "<GMT: Timestamp>"
    },
}
```

#### Parameters in binary

```
Parameters ::= (token: Token) (expiry: Timestamp)

```

### Function `authorize`

Full name: `BictoryAuction.authorize`

After initialization, this `authorize` function must be called by contract owner's (who is the actual owner of the NFT as well as contract owner of this auction contract) private key to allow this auction contract to transfer NFT after finalization to the highest bidder's address.

#### Logs

Produces `UpdateOperatorEvent`.


### Function `bid`

Full name: `BictoryAuction.bid`

The bid will be placed if sender is calling this functionality with some amount that he/she wants to bid and same money will be deducted from his/her wallet.

#### Logs

Produces `BidingEvent`.


### Function `finalize`

Full name: `BictoryAuction.finalize`

The `finalize` function can only be called when the current GMT time will greater than expiry time provided dring contract initialisation.

#### Logs

Produces `Finalize`.


### Function `cancel`

Full name: `BictoryAuction.cancel`

This function can be called anytime to cancel the existing auction. After this operation the money will be returned to all bidders.

#### Logs

Produces `Cancel`.


### Function `view`

Full name: `BictoryAuction.view`

This function can be called anytime to view the existing auction.

#### Return value

```
AuctionState ::= (tag: u8 = 0; NotSoldYet)
               | (tag: u8 = 1; Sold) (owner: AccountAddress)
               | (tag: u8 = 2; Canceled)

Result ::= (item: Token) (expiry: Timestamp) (auction_state: AuctionState) (highest_bid: Amount) (is_authorised: Boolean)
```