//! Use format strings to create strongly-typed data pack/unpack interfaces (inspired by Python's `struct` library).
//!
//!
//! # Installation
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! structure = "0.1"
//! ```
//!
//! And this to your crate root:
//!
//! ```rust
//! #[macro_use]
//! extern crate structure;
//!
//! # fn main() {}
//! ```
//!
//! # Examples
//!
//! ```rust
//! # #[macro_use]
//! # extern crate structure;
//! # fn foo() -> std::io::Result<()> {
//! // Two `u32` and one `u8`
//! let s = structure!("2IB");
//! let buf: Vec<u8> = s.pack(1, 2, 3)?;
//! assert_eq!(buf, vec![0, 0, 0, 1, 0, 0, 0, 2, 3]);
//! assert_eq!(s.unpack(buf)?, (1, 2, 3));
//! # Ok(())
//! # }
//! # fn main() {
//!     # foo().unwrap();
//! # }
//! ```
//!
//! It's useful to use `pack_into` and `unpack_from` when using types that implement `Write` or `Read`.
//! The following example shows how to send a `u32` and a `u8` through sockets:
//!
//! ```rust
//! # #[macro_use]
//! # extern crate structure;
//! # fn foo() -> std::io::Result<()> {
//! use std::net::{TcpListener, TcpStream};
//! let listener = TcpListener::bind("127.0.0.1:0")?;
//! let mut client = TcpStream::connect(listener.local_addr()?)?;
//! let (mut server, _) = listener.accept()?;
//! let s = structure!("IB");
//! s.pack_into(&mut client, 1u32, 2u8)?;
//! let (n, n2) = s.unpack_from(&mut server)?;
//! assert_eq!((n, n2), (1u32, 2u8));
//! # Ok(())
//! # }
//! # fn main() {
//!     # foo().unwrap();
//! # }
//! ```
//!
//! # Format Strings
//!
//! ## Endianness
//!
//! By default, the endianness is big-endian. It could be determined by specifying one of the
//! following characters at the beginning of the format:
//!
//! Character   |   Endianness
//! ---------   |   ----------
//! '='         |   native (target endian)
//! '<'         |   little-endian
//! '>'         |   big-endian
//! '!'         |   network (= big-endian)
//!
//! ## Types
//!
//! Character   |   Type
//! ---------   |   ----
//! 'b'         |   `i8`
//! 'B'         |   `u8`
//! '?'         |   `bool`
//! 'h'         |   `i16`
//! 'H'         |   `u16`
//! 'i'         |   `i32`
//! 'I'         |   `u32`
//! 'q'         |   `i64`
//! 'Q'         |   `u64`
//! 'f'         |   `f32`
//! 'd'         |   `f64`
//! 's'         |   `&[u8]`
//! 'S'         |   `&[u8]`
//! 'P'         |   `*const c_void`
//! 'x'         |   padding (1 byte)
//!
//! * Any format character may be preceded by an integral repeat count. For example, the format string '4h'
//! means exactly the same as 'hhhh'.
//! * 'P' may be follow by a `<type>`, so `"P<u32>"` means a pointer to u32 (`*const u32`).
//! * When 's' is packed, its value can be smaller than the size specified in the format,
//! and the rest will be filled with zeros. For instance:
//!
//! ```rust
//! # #[macro_use]
//! # extern crate structure;
//! # fn foo() -> std::io::Result<()> {
//! assert_eq!(structure!("3s").pack(&[8, 9])?, vec![8, 9, 0]);
//! # Ok(())
//! # }
//! # fn main() {
//!     # foo().unwrap();
//! # }
//! ```
//!
//! * Unlike 's', 'S' is a fixed-size buffer, so the size of its value must be exactly the size
//! specified in the format.
//! * By default, 's' and 'S' are buffers of one byte. To create a fixed-sized buffer with ten bytes,
//! the format would be "10S".
//! * On unpack, 'x' skips a byte. On pack, 'x' always writes a null byte. To skip multiple bytes,
//! prepend the length like in "10x".
//!
//! # Differences from Python struct library
//!
//! While the format strings look very similar to Python's `struct` library, there are a few differences:
//!
//! * Numbers' byte order is big-endian by default (e.g. u32, f64...).
//! * There is no alignment support.
//! * In addition to 's' (buffer) format character, that when packed, its value can be smaller than
//! the size specified in the format, there is the 'S' format character, that the size of its value must
//! be exactly the size specified in the format.
//! * The type of a pointer is `c_void` by default, but can be changed.
//! * 32 bit integer format character is only 'I'/'i' (and not 'L'/'l').
//! * structure!() macro takes a literal string as an argument.
//! * It's called `structure` because `struct` is a reserved keyword in Rust.

#![cfg_attr(not(feature = "std"), no_std)]

#[doc(hidden)]
pub extern crate byteorder;

#[doc(hidden)]
pub use structure_macro_impl::structure;
