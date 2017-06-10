# Structure

**Use format strings to create strongly-typed data pack/unpack interfaces (inspired by Python's `struct` library).**

[![Build Status](https://travis-ci.org/liranringel/structure.svg?branch=master)](https://travis-ci.org/liranringel/structure)
[![Build status](https://ci.appveyor.com/api/projects/status/tiwjo6q4eete0nmh/branch/master?svg=true)](https://ci.appveyor.com/project/liran-ringel/structure/branch/master)
[![Crates.io](https://img.shields.io/crates/v/structure.svg)](https://crates.io/crates/structure)

[Documentation](https://docs.rs/structure)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
structure = "0.1"
```

And this to your crate root:

```rust
#[macro_use]
extern crate structure;
```

## Examples

```rust
// Two `u32` and one `u8`
let s = structure!("2IB");
let buf: Vec<u8> = s.pack(1, 2, 3)?;
assert_eq!(buf, vec![0, 0, 0, 1, 0, 0, 0, 2, 3]);
assert_eq!(s.unpack(buf)?, (1, 2, 3));
```

It's useful to use `pack_into` and `unpack_from` when using types that implement `Write` or `Read`.
The following example shows how to send a `u32` and a `u8` through sockets:

```rust
use std::net::{TcpListener, TcpStream};
let listener = TcpListener::bind("127.0.0.1:0")?;
let mut client = TcpStream::connect(listener.local_addr()?)?;
let (mut server, _) = listener.accept()?;
let s = structure!("IB");
s.pack_into(&mut client, 1u32, 2u8)?;
let (n, n2) = s.unpack_from(&mut server)?;
assert_eq!((n, n2), (1u32, 2u8));
```

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
