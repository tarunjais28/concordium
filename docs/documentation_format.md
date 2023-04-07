# Documentation format

## Overview

Contract documentation has following sections:

* Specifications - Specifications and standards that contract implements. See corresponding spec documentation for
  function descriptions;
* Types - Common types that are used through various contract functions;
* Events - Log binary format, that is used across contract functions;
* Functions:
  * Initialization - Contract initialization function;
  * Read functions - read only contract functions, that do not affect instance state;
  * Write functions - write contract functions, that affect instance state.

Each contract function documentation has the following format:

* Name and description;
* Parameters in binary;
* Return value;
* Events;
* Errors.


## Binary format

Following format is used to describe binary data:

* `::=` assigns name to a byte group that follows it;
* `()` logical group of bytes;
* `:` separates group name on the left and group type on the right;
* `|` alternative group variant, usually used with tag;
* `=` indicates allowed value this variant;
* `u8, u16, u32, u64` number types, `u` for unsigned, `i` for signed, number indicates number size in bits;
* `LE` number is expected in little endian format. Least significant byte comes first, most significant comes last;
* `*` inside logical group indicates how many times it should be repeated;
* `//` comment separator, does not affect data format, ends at line break;
* `;` comment separator within a logical group, does not affect data format. Ends at the bracket that closes the group.

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
