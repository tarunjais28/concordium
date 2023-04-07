# CNS NFT contract internal API

## Specifications

* [Authority](../../../../docs/specs/authority.md)


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
Duration ::= (milliseconds: u64 as LE)
```

```
UpdateOperation ::= (tag: u8 = 0; Remove)
                  | (tag: u8 = 1; Add)
```

```
Percentage ::= (micro_percent: u64)
```


## Initialization

Full name: `init_BictoryCnsNft`

#### Parameters in binary

```
Parameters ::= (storage_contract: ContractAddress) (royalty_on_mint: Percentage) (grace_on_mint: Duration) (beneficiary: AccountAddress)
```


## Write functions

### Function `mint`

Receive name: `BictoryCnsNft.mint`

Only authorized CNS contract is allowed to call this function.

Create a new token for a registered domain.

#### Parameters in binary

```
Parameters ::= (token_id: TokenId) (domain: String) (owner: Address) (duration: Duration)
```

#### Events

* `CIS-1 MintEvent`


### Function `lend`

Receive name: `BictoryCnsNft.lend`

Only authorized CNS contract is allowed to call this function.

Called by CNS contract after processing payment information to extend subscription period for the owner.

#### Parameters in binary

```
Parameter ::= (token_id: TokenId) (extension: Duration)
```


### Function `updateInternalValue`

Receive name: `BictoryCnsNft.updateInternalValue`

Requires maintainer rights or higher.

Update values requred for internal contract functioning.

#### Parameters in binary

```
Parameter ::= (tag: u8 = 0; CnsContract) (update: UpdateOperation) (address: ContractAddress)
            | (tag: u8 = 1; Royalty) (royalty: Percentage)
            | (tag: u8 = 2; Beneficiary) (address: AccountAddress)
```


## Read functions

### Function `viewInternalValue`

Receive name: `BictoryCnsNft.viewInternalValue`

View values requred for internal contract functioning. Since parameter and return value size are limited, `skip`
and `show` parameters are required for CNS contract list.

* `skip` - the amount of addresses to skip when returning the address list;
* `show` - the maximum amount of addresses to include in the returned list.

#### Parameters in binary

```
Parameter ::= (tag: u8 = 0; CnsContract) (skip: u32) (show: u32)
            | (tag: u8 = 1; Royalty)
            | (tag: u8 = 2; Beneficiary)
```

#### Return value

```
Result ::= (tag: u8 = 0; CnsContract) (length: u32) (addresses: ContractAddress * length)
         | (tag: u8 = 1; Royalty) (royalty: Percentage)
         | (tag: u8 = 2; Beneficiary) (beneficiary: AccountAddress)
```
