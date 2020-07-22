//! cacache is a Rust library for managing local key and content address
//! caches. It's really fast, really good at concurrency, and it will never
//! give you corrupted data, even if cache files get corrupted or manipulated.
//!
//! ## API Layout
//!
//! The cacache API is organized roughly similar to `std::fs`; most of the
//! toplevel functionality is available as free functions directly in the
//! `cacache` module, with some additional functionality available through
//! returned objects, as well as `WriteOpts`, which is analogous to
//! `OpenOpts`, but is only able to write.
//!
//! One major difference is that the default APIs are all async functions, as
//! opposed to `std::fs`, where they're all synchronous. Synchronous APIs in
//! cacache are accessible through the `_sync` suffix.
//!
//! ### Suffixes
//!
//! You may notice various suffixes associated with otherwise familiar
//! functions:
//!
//! * `_sync` - Most cacache APIs are asynchronous by default. Anything using
//!   the `_sync` suffix behaves just like its unprefixed counterpart, except
//!   the operation is synchronous.
//! * `_hash` - Since cacache is a content-addressable cache, the `_hash`
//!   suffix means you're interacting directly with content data, skipping the
//!   index and its metadata. These functions use an `Integrity` to look up
//!   data, instead of a string key.
//!
//! ## Examples
//!
//! Un-suffixed APIs are all async, using
//! [`async-std`](https://crates.io/crates/async-std). They let you put data
//! in and get it back out -- asynchronously!
//!
//! ```no_run
//! use async_attributes;
//!
//! #[async_attributes::main]
//! async fn main() -> cacache::Result<()> {
//!   // Data goes in...
//!   cacache::write("./my-cache", "key", b"hello").await?;
//!
//!   // ...data comes out!
//!   let data = cacache::read("./my-cache", "key").await?;
//!   assert_eq!(data, b"hello");
//!
//!   Ok(())
//! }
//! ```
//!
//! ### Lookup by hash
//!
//! What makes `cacache` content addressable, though, is its ability to fetch
//! data by its "content address", which in our case is a ["subresource
//! integrity" hash](https://crates.io/crates/ssri), which `cacache::put`
//! conveniently returns for us. Fetching data by hash is significantly faster
//! than doing key lookups:
//!
//! ```no_run
//! use async_attributes;
//!
//! #[async_attributes::main]
//! async fn main() -> cacache::Result<()> {
//!   // Data goes in...
//!   let sri = cacache::write("./my-cache", "key", b"hello").await?;
//!
//!   // ...data gets looked up by `sri` ("Subresource Integrity").
//!   let data = cacache::read_hash("./my-cache", &sri).await?;
//!   assert_eq!(data, b"hello");
//!
//!   Ok(())
//! }
//! ```
//!
//! ### Large file support
//!
//! `cacache` supports large file reads, in both async and sync mode, through
//! an API reminiscent of `std::fs::OpenOptions`:
//!
//! ```no_run
//! use async_attributes;
//! use async_std::prelude::*;
//!
//! #[async_attributes::main]
//! async fn main() -> cacache::Result<()> {
//!   let mut fd = cacache::Writer::create("./my-cache", "key").await?;
//!   for _ in 0..10 {
//!     fd.write_all(b"very large data").await.expect("Failed to write to cache");
//!   }
//!   // Data is only committed to the cache after you do `fd.commit()`!
//!   let sri = fd.commit().await?;
//!   println!("integrity: {}", &sri);
//!
//!   let mut fd = cacache::Reader::open("./my-cache", "key").await?;
//!   let mut buf = String::new();
//!   fd.read_to_string(&mut buf).await.expect("Failed to read to string");
//!
//!   // Make sure to call `.check()` when you're done! It makes sure that what
//!   // you just read is actually valid. `cacache` always verifies the data
//!   // you get out is what it's supposed to be. The check is very cheap!
//!   fd.check()?;
//!
//!   Ok(())
//! }
//! ```
//!
//! ### Sync API
//!
//! There are also sync APIs available if you don't want to use async/await.
//! The synchronous APIs are generally faster for linear operations -- that
//! is, doing one thing after another, as opposed to doing many things at
//! once. If you're only reading and writing one thing at a time across your
//! application, you probably want to use these instead.
//!
//! ```no_run
//! fn main() -> cacache::Result<()> {
//!   cacache::write_sync("./my-cache", "key", b"my-data").unwrap();
//!   let data = cacache::read_sync("./my-cache", "key").unwrap();
//!   assert_eq!(data, b"my-data");
//!   Ok(())
//! }
//! ```
#![warn(missing_docs, missing_doc_code_examples)]

pub use serde_json::Value;
pub use ssri::Algorithm;

mod content;
mod errors;
mod index;

mod get;
mod ls;
mod put;
mod rm;

pub use errors::{Error, Result};
pub use index::Metadata;

pub use get::*;
pub use ls::*;
pub use put::*;
pub use rm::*;
