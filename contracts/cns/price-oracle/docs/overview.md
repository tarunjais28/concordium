# CNS Price oracle

Set and read pricing policy for CNS domains.


## Types

```
DomainKind ::= (tag: u8 = 0; Domain)
             | (tag: u8 = 1; Subdomain)
```

```
DomainPrice ::= (tag: u8 = 0; Limited)
              | (tag: u8 = 1; Amount) (micro_ccd: u64)
```


## Read functions

### Function `getYearlyDomainPrice`

Receive name: `BictoryCnsPriceOracle.getYearlyDomainPrice`

Get pricing info for domain with given parameters.

#### Parameters in binary

```
Parameter ::= (domain_kind: DomainKind) (length: u16)
```

#### Return value

```
Result ::= (result: DomainPrice)
```

#### Errors

* `-2147483646 ParseError`
