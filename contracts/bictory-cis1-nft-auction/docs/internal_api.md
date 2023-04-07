# Auction

Contract name: `BictoryNftAuction`

## Specifications

* [Authority](../../../docs/specs/authority.md)

## Types

```
AccountAddress ::= (address: u8 * 32)
```

```
Percentage ::= (micro_percent: u64 as LE)
```


## Initialization

Full name: `init_BictoryNftAuction`

Initialize BictoryNftAuction contract with beneficiary for this marketplace and it's royalty percentage. Any amount
of tokens can be auctioned on a single auction contract instance.

#### Parameters in binary

```
Parameter ::= (beneficiary: AccountAddress) (percentage: Percentage)
```


## Write functions

### Function `updateInternalValue`

Receive name: `BictoryNftAuction.updateInternalValue`

Requires maintainer rights or higher.

Update values requred for internal contract functioning.

#### Parameters in binary

```
Parameter ::= (tag: u8 = 0; Royalty) (royalty: Percentage)
            | (tag: u8 = 1; Beneficiary) (address: AccountAddress)
```

#### Errors

* `-36 Unauthorized`
* `-2147483646 ParseError`


## Read functions

### Function `viewInternalValue`

Receive name: `BictoryNftAuction.viewInternalValue`

View values requred for internal contract functioning.

#### Parameters in binary

```
Parameter ::= (tag: u8 = 0; Royalty)
            | (tag: u8 = 1; Beneficiary)
```

#### Return value

```
Result ::= (tag: u8 = 0; Royalty) (royalty: Percentage)
         | (tag: u8 = 1; Beneficiary) (address: AccountAddress)
```

#### Errors

* `-2147483646 ParseError`
