# Authority

Authorised account management. Authority module can be used by the contract to update contract values that affect it's
logic or for any other purpose. Functions that require certain authority levels should be documented by each
conforming contract.

## Types

```
AuthorityField ::= (tag: u8 = 0; Maintainer)
                 | (tag: u8 = 1; Admin)
```

```
UpdateOperation ::= (tag: u8 = 0; Remove)
                  | (tag: u8 = 1; Add)
```

```
AccountAddress ::= (address: u8 * 32)
```

```
ContractAddress ::= (index: u64 as LE) (subindex: u64 as LE)
```

```
Address ::= (tag: u8 = 0) (address: AccountAddress)
          | (tag: u8 = 1) (address: ContractAddress)
```


## Read functions

### Function `viewAuthority`

Receive name: `BictoryNftAuction.viewAuthority`

View admin or maintainer list. Since parameter and return value size are limited, `skip` and `show` parameters are
required.

* `skip` - the amount of addresses to skip when returning the address list;
* `show` - the maximum amount of addresses to include in the returned list.

#### Parameters in binary

```
Parameter ::= (field: AuthorityField) (skip: u32 as LE) (show: u32 as LE)
```

#### Return value

```
Result ::= (length: u32 as LE) (addresses: Address * length)
```

#### Errors

* `-2147483646 ParseError`
  * Invalid function parameters.


## Write functions

### Function `updateAuthority`

Entrypoint name: `updateAuthority`

Requires maintainer rights or higher.

Update admin or maintainer list.

#### Parameters in binary

```
Parameter ::= (field: AuthorityField) (update_kind: UpdateOperation) (address: Address)
```

#### Errors

* `-2147483646 ParseError`
  * Invalid function parameters.
* `-36 Unauthorized`
  * Attempt to update maintainer list as an unknown account;
  * Attempt to update admin list as an unknown or maintainer account.
