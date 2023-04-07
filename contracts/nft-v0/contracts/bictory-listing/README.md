# Listing

Contract name: `BictoryListing`


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
Amount ::= (micro_ccd: u64 as LE)
```

```
Royalty ::= (micro_percent: u32 as LE)
```

```
Listing ::= (token: Token) (owner: AccountAddress) (creator: AccountAddress) (creator_royalty: Royalty) (minter: AccountAddress) (minter_royalty: Royalty) (price: Amount) (for_sale: Boolean)
```

```
Token ::= (contract: ContractAddress) (id: TokenId)
```

```
Boolean ::= (bool: u8 = 0; false) 
          | (bool: u8 = 1; true)
```

### Logs

```
ListingEvent ::= (tag: u8 = 247) (for_sale: Boolean) (listing: Listing)
```

```
UnlistingEvent ::= (tag: u8 = 249) (token: Token) (for_sale: Boolean)
```

```
BuyEvent ::= (tag: u8 = 248) (token: Token) (seller: AccountAddress) (buyer: AccountAddress) ( owner_share: Amount) (creator_share: Amount) (for_sale: Boolean)
```


## Function paramters

### Function `list`

Full name: `BictoryListing.list`

This function is used to list NFT for sale. After this operation the `for_sale` flag will be set to `true` and throws `TokenAlreadyListedForSale` error in case the flag is already in `true` state. Before this operation the contract must be initialized for particular NFTs which is going to be listed for sale provided that all NFTs belong to same owner. After this the contract_address of this initialized contract must be set as the operator to the given NFT contract.

#### Steps:
1. Initialise contract for listing
2. List NFT for sale
3. Now buyer can buy or Owner can remove listing
4. This listing contract can be reused by the NFT owner for listing another token or if not then Operator must be removed by calling `BictoryNFT.updateOperator` by owner's private key

#### Parameters as JSON

```
{
    "creator": "<wallet_address: string>",
    "token": {
        "contract": {
            "index": <instance_index: number>,
            "subindex": <instance_subindex: number>
        },
        "id": {<token_id: array of u8>}
    },
    "owner": "<wallet_address: string>",
    "creator_royalty": <royalty_percentage: u32 (units: 1/1000000 %)>,
    "minter": "<wallet_address: string>",
    "minter_royalty": <royalty_percentage: u32 (units: 1/1000000 %)>,
    "price": <price: string (units: microCCD)>,
    "for_sale": Boolean
}
```

#### Parameters in binary

```
Parameter ::= (token: Token) (owner: AccountAddress) (creator: AccountAddress) (creator_royalty: Royalty) (minter: AccountAddress) (minter_royalty: Royalty) (price: Amount) (for_sale: Boolean)
```

#### Logs

Produces `ListingEvent` per each listed token.


### Function `unlist`

Full name: `BictoryListing.unlist`

This function is used to unlist token for sale. After this operation the `for_sale` flag in the target NFT contract will be set to `false`.

#### Parameters as JSON

```
{
    "token": {
        "contract": {
            "index": <instance_index: number>,
            "subindex": <instance_subindex: number>
        },
        "id": {<token_id: array of u8>}
    },
    "owner": "<wallet_address: string>"
}
```

#### Parameters in binary

```
Parameter ::= (token: Token) (owner: AccountAddress)
```

#### Logs

Produces `UnlistingEvent` per each transferred token.


### Function `buy`

Full name: `BictoryListing.buy`

This function used to buy listed NFT and after the transfer of token all balances will be transfered to respective accounts of `Minter`, `Creator`, `Bictory` and `Sellers`.

#### Parameters as JSON

```
{
    "token": {
        "contract": {
            "index": <instance_index: number>,
            "subindex": <instance_subindex: number>
        },
        "id": {<token_id: array of u8>}
    },
    "bictory_royalty": <royalty_percentage: u32 (units: 1/1000000 %)>
},
```

#### Parameters in binary

```
Parameter ::= (token: Token) (bictory_royalty: Royalty)
```

#### Logs

Produces `BuyEvent`.


### Function `updatePrice`

Full name: `BictoryListing.updatePrice`

#### Parameters as JSON

```
{
    "token": {
        "contract": {
            "index": <instance_index: number>,
            "subindex": <instance_subindex: number>
        },
        "id": {<token_id: array of u8>}
    },
    "price": <price: string (units: microCCD)>
}
```

#### Parameters in binary

```
Parameter ::= (token: Token) (price: Amount)
```

#### Logs

Produces `UpdatePriceEvent`
