# NFT

Contract name: `BictoryNFT`


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
Receiver ::= (tag: u8 = 0) (address: AccountAddress)
           | (tag: u8 = 1) (address: ContractAddress) (hook: ReceiveHookName)
```

```
ReceiveHookName ::= (size: u16 as LE) (name: u8 as ASCII * size; format: <contract>.<function>)
```

```
Amount ::= (micro_ccd: u64 as LE)
```

```
Royalty ::= (micro_percent: u32 as LE)
```

```
IpfsCid ::= (size: u32 as LE) (buffer: u8 * size)
```

```
Sha256Hash ::= (size: u32 as LE = 0)
             | (size: u32 as LE = 32) (buffer: u8 * 32)
```

### Logs

```
TransferEvent ::= (tag: u8 = 255) (token_id: TokenId) (token_amount: u64; 0 or 1, 0 does not change state) (from: Address) (to: Address)
```

```
MintEvent ::= (tag: u8 = 254) (token_id: TokenId) (price: Amount) (owner: Address)
```

```
BurnEvent ::= (tag: u8 = 253) (token_id: TokenId) (token_amount: u64; 0 or 1, 0 does not change state) (owner: Address)
```

```
UpdatePriceEvent ::= (tag: u8 = 240) (token_id: TokenId) (owner: Address) (from: u64) (to: u64)
```

```
UpdateKind ::= (tag: byte = 0; Remove)
             | (tag: byte = 1; Add)

UpdateOperatorEvent ::= (tag: u8 = 252) (kind: UpdateKind) (owner: Address) (operator: Address)
```

```
MetadataUrl ::= (size: u16 as LE) (url: u8 as ASCII * size) (has_hash: u8 = 0)

TokenMetadataEvent ::= (tag: u8 = 251) (token_id: TokenId) (metadata_url: MetadataUrl)
```


## Function paramters

### Initialization

Full name: `init_BictoryNFT`

#### Parameters as JSON

```
{
    "index": <instance_index: number>,
    "subindex": <instance_subindex: number>
}
```

#### Parameters in binary

```
Parameters ::= (storage_address: ContractAddress)

```

### Function `mint`

Full name: `BictoryNFT.mint`

#### Parameters as JSON

```
{
    "mint_data": [
        {
            "token_id": <token_id: array of u8>,
            "creator": {
                "Account": [
                    <wallet_address: string>
                ]
            },
            "creator_royalty": <royalty_percentage: u32 (units: 1/1000000 %)>,
            "minter_royalty": <royalty_percentage: u32 (units: 1/1000000 %)>,
            "price": <price: string (units: microCCD)>,
            "cid": <IPFS concent ID: array of u8>
        },
        ...
    ]
}
```

#### Parameters in binary

```
MintData ::= (token_id: TokenId) (creator: Address) (creator_royalty: Royalty) (minter_royalty: Royalty) (price: Amount) (cid: IpfsCid)

Parameter ::= (length: u32 as LE) (mint_data: MintData * length)
```

#### Logs

Produces `MintEvent` and `TokenMetadataEvent` per each minted token.


### Function `transfer`

Full name: `BictoryNFT.transfer`

#### Parameters as JSON

```
[
    {
        "token_id": <token_id: array of u8>,
        "from": {
            "Account": [
                <wallet_address: string>
            ]
            OR
            "Contract": [
                {
                    "index": <instance_index: number>,
                    "subindex": <instance_subindex: number>
                }
            ]
        },
        "to": {
            "Account": [
                <wallet_address: string>
            ]
            OR
            "Contract": [
                {
                    "index": <instance_index: number>,
                    "subindex": <instance_subindex: number>
                }
            ]
        },
        "data": [],
        "amount": <amount: number>
    },
    ...
]
```

#### Parameters in binary

```
Transfer ::= (token_id: TokenId) (amount: u64 as LE) (from: Address) (to: Receiver) (data = [0, 0])

Parameter ::= (length: u16 as LE) (transfer_data: Transfer * length)
```

#### Notes
To set `for_sale` flag `false`, put first byte of `data` field must be set as `0` or empty, for rest 
of the values of first byte of `data` field, `for_sale` will be `true`. 

#### Logs

Produces `TransferEvent` per each transferred token.


### Function `updateOperator`

Full name: `BictoryNFT.updateOperator`

#### Parameters as JSON

```
[
    [
        {
            "operator": {
                "Account": [
                    <wallet_address: string>
                ]
                OR
                "Contract": [
                    {
                        "index": <instance_index: number>,
                        "subindex": <instance_subindex: number>
                    }
                ]
            },
            "update": {
                "Remove": []
                OR
                "Add": []
            }
        },
        ...
    ]
]
```

#### Parameters in binary

```
UpdateKind ::= (tag: byte = 0; Remove)
             | (tag: byte = 1; Add)

UpdateOperator ::= (kind: UpdateKind) (operator: Address)

Parameter ::= (length: u16 as LE) (update_data: UpdateOperator * length)
```

#### Logs

Produces `UpdateOperatorEvent` per each update.


### Function `operatorOf`

Full name: `BictoryNFT.operatorOf`

#### Parameters as JSON

```
{
    "result_contract": {
                "index": <instance_index: number>,
                "subindex": <instance_subindex: number>
    },
    "result_function": {
        "contract": <contract_name: string>,
        "func": <contract_function: string>
    },
    "queries": [
        {
            "owner": {
                "Account": [
                    <wallet_address: string>
                ]
                OR
                "Contract": [
                    {
                        "index": <instance_index: number>,
                        "subindex": <instance_subindex: number>
                    }
                ]
            },
            "address": {
                "Account": [
                    <wallet_address: string>
                ]
                OR
                "Contract": [
                    {
                        "index": <instance_index: number>,
                        "subindex": <instance_subindex: number>
                    }
                ]
            }
        },
        ...
    ]
}
```

#### Parameters in binary

```
OperatorOfQuery ::= (owner: Address) (address: Address)

Parameter ::= (result_contract: ContractAddress) (result_function: ReceiveHookName) (length: u16 as LE) (queries: OperatorOfQuery * length)
```

#### Response

```
OperatorOfQueryResult ::= (query: OperatorOfQuery) (is_operator: bool)

Response ::= (length: u16 as LE) (responses: OperatorOfQueryResponse * length)
```


### Function `balanceOf`

Full name: `BictoryNFT.balanceOf`

#### Parameters as JSON

```
{
    "result_contract": {
                "index": <instance_index: number>,
                "subindex": <instance_subindex: number>
    },
    "result_function": <function_name: string>,
    "queries": [
        {
            "token_id": <token_id: array of u8>,
            "address": {
                "Account": [
                    <wallet_address: string>
                ]
                OR
                "Contract": [
                    {
                        "index": <instance_index: number>,
                        "subindex": <instance_subindex: number>
                    }
                ]
            }
        },
        ...
    ]
}
```

#### Parameters in binary

```
BalanceOfQuery ::= (token_id: TokenId) (address: Address)

Parameter ::= (result_contract: ContractAddress) (result_function: ReceiveHookName) (length: u16 as LE) (queries: BalanceOfQuery * length)
```

#### Response

```
BalanceOfQueryResult ::= (query: BalanceOfQuery) (token_amount: u64; 0 or 1)

Response ::= (length: u16 as LE) (responses: OperatorOfQueryResponse * length)
```


### Function `burn`

Full name: `BictoryNFT.burn`

#### Parameters as JSON

```
<token_id: array of u8>
```

#### Parameters in binary

```
Parameter ::= (token_id: TokenId)
```

#### Logs

Produces `BurnEvent` and `TokenMetadataEvent`


### Function `updatePrice`

Full name: `BictoryNFT.updatePrice`

#### Parameters as JSON

```
{
    "token_id": <token_id: array of u8>,
    "price": <price: string (units: microCCD)>
}
```

#### Parameters in binary

```
Parameter ::= (token_id: TokenId) (price: Amount)
```

#### Logs

Produces `UpdatePriceEvent` and `TokenMetadataEvent`
