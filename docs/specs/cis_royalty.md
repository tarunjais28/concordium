# CIS Royalty extension

CIS-1 and CIS-2 extension. Reports all the royalties associated with a specific token.

## Types

```
TokenId ::= (size: u8) (buffer: u8 * size)
```

```
Percentage ::= (micro_percent: u64 as LE)
```

```
Royalty ::= (beneficiary: AccountAddress) (percentage: Percentage)
```


## Read functions

### Function `getRoyalties`

Entrypoint name: `getRoyalties`

Return a list of royalties associated with given token ID.

#### Parameters in binary

```
Parameter ::= (token_id: TokenId)
```

#### Return value

```
Result ::= (length: u32 as LE) (royalties: Royalty * length)
```

#### Errors

* `-2147483646 ParseError`
  * Invalid function parameters.
* `-42000001 InvalidTokenId`
  * Attempt to read royalty for unknown token.
* Non exhaustive
  * Other errors may be returned according to contract logic.
