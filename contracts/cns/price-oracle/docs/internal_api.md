# CNS Price oracle

Allows to set and read pricing policy for CNS domains.

Two pricing policies are supported for domains by their length: fixed and scaling. Fixed pricing policy means equal
price for domains and subdomains of any length. Scaling policy allows to specify price scaling in detail. It is
described in `ScalingPricing` data type.

### `DomainPrice` description

* `Limited` - domain registration with given length is only allowed to be performed by user with maintainer rights;
* `Amount` - domain registration with given length is allowed to everyone for specified amount.

### `ScalingPricing` description

* `short_max_length` - maximum character length for domain to be considered short, `short` domain price applies for
  all domains no longer than this length;
* `short` - price that applies to domains no longer than `short_max_length`;
* `mid` - ordered array of prices for each extra UTF-8 character in total domain length over `short_max_length`, one by
  one;
* `long` - price that applies to domains longer than `short_max_length` + length of `mid` array.


## Specifications

* [Authority](../../../../docs/specs/authority.md)


## Types

```
DomainPrice ::= (tag: u8 = 0; Limited)
              | (tag: u8 = 1; Amount) (micro_ccd: u64)
```

```
DomainPricing ::= (tag: u8 = 0; Fixed) (price: DomainPrice)
                | (tag: u8 = 1; Scaling) (pricing: ScalingPricing)
```

```
PricingList ::= (count: u64) (prices: DomainPrice * count)

ScalingPricing ::= (short_max_length: u16) (short: DomainPrice) (mid: PricingList) (long: DomainPrice)
```


## Events

```
SetYearlyDomainPriceEvent ::= (domain_pricing: DomainPricing) (subdomain_pricing: DomainPricing)
```


## Write functions

### Function `setYearlyDomainPrice`

Receive name: `BictoryCnsPriceOracle.setYearlyDomainPrice`

Requires maintainer rights or higher.

Set pricing info for domain or subdomain.

#### Parameters in binary

```
Parameter ::= (domain_pricing: DomainPricing) (subdomain_pricing: DomainPricing)
```

#### Events

* `SetYearlyDomainPriceEvent`

#### Errors

* `-36 Unauthorized`
  * Caller doesn't have maintainer nor admin rights.
* `-2147483646 ParseError`
