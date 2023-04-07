# CNS

## Specifications

* [Authority](../../../../docs/specs/authority.md)

## Types

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
TokenId ::= (size: u8) (buffer: u8 * size)
```

```
DataValue ::= (tag: u8 = 0; Empty)
            | (tag: u8 = 1; Address) (value: Address)
            | (tag: u8 = 2; URL)     (url: String)
            | (tag: u8 = 3; Binary)  (bytes: Bytes)
            | (tag: u8 = 4; String)  (string: String)
            | (tag: u8 = 5; TokenId) (contract: ContractAddress) (id: TokenId)
```

```
AuthorityField ::= (tag: u8 = 0; Maintainer)
                 | (tag: u8 = 1; Admin)
```

```
AuthorityUpdateKind ::= (tag: u8 = 0; Remove)
                      | (tag: u8 = 1; Add)
```


## Initialization

Full name: `init_BictoryCns`

#### Parameters in binary

```
Parameter ::= (registry: ContractAddress) (nft: ContractAddress) (price_oracle: ContractAddress) (subscription_year_limit: u8)
```


## Write functions

### Function `updateInternalValue`

Full name: `BictoryCns.updateInternalValue`

Requires maintainer rights or higher.

Update CNS NFT, price oracle or beneficiary addresses.

#### Parameters in binary

```
Parameter ::= (tag: u8 = 0; CNS NFT)     (address: ContractAddress)
            | (tag: u8 = 1; Oracle)      (address: ContractAddress)
            | (tag: u8 = 2; Beneficiary) (address: AccountAddress)
            | (tag: u8 = 3; Subscription limit) (years: u8)
```


## Read functions

### Function `viewInternalValue`

Receive name: `BictoryCnsNft.viewInternalValue`

View values required for internal contract functioning.

#### Parameters in binary

```
Parameter ::= (tag: u8 = 0; CNS NFT)
            | (tag: u8 = 1; Oracle)
            | (tag: u8 = 2; Beneficiary)
            | (tag: u8 = 3; Subscription limit)
```

#### Return value

```
Result ::= (tag: u8 = 0; CNS NFT)     (address: ContractAddress)
         | (tag: u8 = 1; Oracle)      (address: ContractAddress)
         | (tag: u8 = 2; Beneficiary) (address: AccountAddress)
         | (tag: u8 = 3; Subscription limit) (years: u8)
```
