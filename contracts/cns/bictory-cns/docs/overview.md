# CNS

CNS contract is a central contract with core CNS logic. It is responsible for registering domains, extending
subscriptions and updating values that domains resolve to.

All registered names are stored in the registry contract instance. Registry is a `BictoryStorage` v1 key-value storage
contract. Single domain can resolve to various addresses, that user stores. User is allowed to store unlimited amount
of data per each domain. CNS name ownership is entirely managed by CNS NFT contracts. This way name ownership could be
purchased or sold on NFT marketplace.

Registry security is ensured by only allowing explicitly authorized CNS contracts to make changes to it.

### Domain name format

Domain name must be a valid UTF-8 string under 256 bytes long. Labels are separated by `.`, each label must be under 64
bytes long. CNS domains must end in `.ccd`

It is recommended to convert domain names to lowercase on each function call for better interoperability with Bictory
CNS. It is also a up to the caller to make sure domain name doesn't contain characters that are easy to misinterpret
as another one.

### Key format

To allow predictable and optimal domain name search, names are hashed before storing them in registry contract.
Function to hash names is called `namehash` and it is identical to ENS.

#### Algorithm description

1. Split domain names by `'.'`;
2. Reverse resulting list;
3. Hash first element with Keccak256 hashing algorithm;
4. Resulting hash join with next element and apply Keccak256 hashing again;
5. Repeat step 4 until there are no elements left in the list.

Resulting hash can be used as a key in the registry contract.


### Subdomains

Subdomains can be created for personal use, but can not be transfered or traded. This implementation does not mint NFT
tokes for new subdomains.


## Types

```
String ::= (size: u32 as LE) (buffer: u8 * size; UTF-8 encoded)
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


## Write functions

### Function `register`

Receive name: `BictoryCns.register`

If given domain name does not exist or has expired, create a new registry entry and a new CNS NFT token. CNS NFT token
ID is equal to `namehash(domain)`. Any compatible CNS NFT contract address can be used to update it. Total
registration subscription duration is limited by `subscription_year_limit`. This period can be extended, but total
subscription duration can never exceed `subscription_year_limit` years from current date.

#### Parameters in binary

```
Parameter ::= (domain: String) (address: Address) (duration_years: u8)
```

#### Errors

* `-1 ParseError`
* `-31 InvalidDuration`
  * `duration_years` exceeds subscription year limit.
* `-35 AlreadyExists`
  * Domain already exists and has not expired.
* `-36 Unauthorized`
  * Price oracle has set limited policy for domains of this length.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format));
  * `domain` is a subdomain.
* `-2147483635 AmountTooLarge`
  * Not enough funds for chosen duration.

Internal errors, can only happen if the contract was configured incorrectly:

* `-32 OperationNotPermitted`
* `-33 InvokeContractError`
* `-37 Incompatible`
* `-2147483634 MissingAccount`


### Function `extend`

Receive name: `BictoryCns.extend`

Extend the subscription duration for given domain. Extension is allowed for up to total of `subscription_year_limit`
years from current date. Everyone is allowed to extend subscription period for any domain, regardless of ownership.

#### Parameters in binary

```
Parameter ::= (domain: String) (duration_years: u8)
```

#### Errors

* `-1 ParseError`
* `-30 NotFound`
  * Domain to extend subscription does not exist or has expired.
* `-31 InvalidDuration`
  * Sum of extension duration and remaining subscription duration exceeds subscription year limit.
* `-36 Unauthorized`
  * Price oracle has set limited policy for domains of this length.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format));
  * `domain` is a subdomain.
* `-2147483635 AmountTooLarge`
  * Not enough funds for chosen duration.

Internal errors, can only happen if the contract was configured incorrectly:

* `-32 OperationNotPermitted`
* `-33 InvokeContractError`
* `-37 Incompatible`
* `-2147483634 MissingAccount`


### Function `setAddress`

Full name: `BictoryCns.setAddress`

Performs NFT ownership check, after that updates the address in the registry.

#### Parameters in binary

```
Parameter ::= (domain: String) (address: Address)
```

#### Events

AddressChanged

#### Errors

* `-1 ParseError`
* `-30 NotFound`
  * Domain to extend subscription does not exist or has expired.
* `-36 Unauthorized`
  * Domain is not owned by the user that sends request;
  * Domain is in grace period.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format)).

Internal errors, can only happen if the contract was configured incorrectly:

* `-32 OperationNotPermitted`
* `-33 InvokeContractError`
* `-37 Incompatible`


### Function `setData`

Full name: `BictoryCns.setData`

Performs NFT ownership check, after that updates the entry in the registry.

#### Parameters in binary

```
Parameter ::= (domain: String) (key: String; non-empty) (value: DataValue)
```

#### Events

DataChanged

#### Errors

* `-1 ParseError`
* `-30 NotFound`
  * Domain to extend subscription does not exist or has expired.
* `-36 Unauthorized`
  * Domain is not owned by the user that sends request;
  * Domain is in grace period.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format)).

Internal errors, can only happen if the contract was configured incorrectly:

* `-32 OperationNotPermitted`
* `-33 InvokeContractError`
* `-37 Incompatible`


### Function `createSubdomain`

Full name: `BictoryCns.createSubdomain`

Can be called by domain owner to create new subdomain. If given subdomain name does not exist, a new registry entry is
created. Subdomains can not be traded or transfered and expire together with the domain.

#### Parameters in binary

```
Parameter ::= (subdomain: String)
```

#### Errors

* `-1 ParseError`
* `-30 NotFound`
  * Domain to extend subscription does not exist or has expired.
* `-36 Unauthorized`
  * Domain is not owned by the user that sends request;
  * Domain is in grace period.
  * Price oracle has set limited policy for subdomains of this length.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format)).
* `-2147483635 AmountTooLarge`
  * Not enough funds for chosen subdomain length.

Internal errors, can only happen if the contract was configured incorrectly:

* `-32 OperationNotPermitted`
* `-33 InvokeContractError`
* `-37 Incompatible`
* `-2147483634 MissingAccount`


### Function `deleteSubdomain`

Full name: `BictoryCns.deleteSubdomain`

Can be called by domain owner to delete subdomain and all it's registry data. This function can also be called by
anyone if the domain has expired to clean up subdomain data.

#### Parameters in binary

```
Parameter ::= (subdomain: String)
```

#### Errors

* `-1 ParseError`
* `-30 NotFound`
  * Subdomain data doesn't exist.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format));
  * `domain` is not a subdomain.

Internal errors, can only happen if the contract was configured incorrectly:

* `-32 OperationNotPermitted`
* `-33 InvokeContractError`
* `-37 Incompatible`


## Read functions

### Function `resolve`

Full name: `BictoryCns.resolve`

#### Parameters in binary

```
Parameter ::= (domain: String)
```

#### Return value

```
Result ::= (address: Address)
```

#### Errors

* `-1 ParseError`
* `-30 NotFound`
  * Subdomain or subdomain doesn't exist or has expired.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format)).

Internal errors, can only happen if the contract was configured incorrectly:

* `-33 InvokeContractError`
* `-37 Incompatible`


### Function `getData`

Full name: `BictoryCns.getData`

Gets the data corresponding to the given key from the registry.

#### Parameters in binary

```
Parameter ::= (domain: String) (key: String)
```

#### Return value

```
Result ::= (value: DataValue)
```

#### Errors

* `-1 ParseError`
* `-30 NotFound`
  * Subdomain or subdomain doesn't exist or has expired.
* `-38 InvalidDomainFormat`
  * Domain doesn't meet the requirements (See [Domain name format](#domain-name-format)).

Internal errors, can only happen if the contract was configured incorrectly:

* `-33 InvokeContractError`
* `-37 Incompatible`
