# cacache ![CI](https://github.com/zkat/cacache-rs/workflows/CI/badge.svg) ![crates.io](https://img.shields.io/crates/v/cacache.svg)

A high-performance, concurrent, content-addressable disk cache, optimized for async APIs.

## Example

```rust
use cacache;
use async_attributes;

#[async_attributes::main]
async fn main() -> Result<(), cacache::Error> {
    let dir = String::from("./my-cache");

    // Write some data!
    cacache::write(&dir, "key", b"my-async-data").await?;

    // Get the data back!
    let data = cacache::read(&dir, "key").await?;
    assert_eq!(data, b"my-async-data");

    // Clean up the data!
    cacache::rm::all(&dir).await?;
}
```

## Install

Using [`cargo-edit`](https://crates.io/crates/cargo-edit)

`$ cargo add cacache`

Minimum supported Rust version is `1.43.0`.

## Documentation

- [API Docs](https://docs.rs/cacache)

## Features

- First-class async support, using [`async-std`](https://crates.io/crates/async-std) as its runtime. Sync APIs are available but secondary
- `std::fs`-style API
- Extraction by key or by content address (shasum, etc)
- [Subresource Integrity](#integrity) web standard support
- Multi-hash support - safely host sha1, sha512, etc, in a single cache
- Automatic content deduplication
- Atomic content writes even for large data
- Fault tolerance (immune to corruption, partial writes, process races, etc)
- Consistency guarantees on read and write (full data verification)
- Lockless, high-concurrency cache access
- Really helpful, contextual error messages
- Large file support
- Pretty darn fast
- Arbitrary metadata storage
- Cross-platform: Windows and case-(in)sensitive filesystem support
- Punches nazis

## Contributing

The cacache team enthusiastically welcomes contributions and project participation! There's a bunch of things you can do if you want to contribute! The [Contributor Guide](CONTRIBUTING.md) has all the information you need for everything from reporting bugs to contributing entire new features. Please don't hesitate to jump in if you'd like to, or even ask us questions if something isn't clear.

All participants and maintainers in this project are expected to follow [Code of Conduct](CODE_OF_CONDUCT.md), and just generally be excellent to each other.

Happy hacking!

## License

This project is licensed under [the Parity License](LICENSE.md). Third-party contributions are licensed under Apache-2.0 and belong to their respective authors.

The Parity License is a copyleft license that, unlike the GPL family, allows you to license derivative and connected works under permissive licenses like MIT or Apache-2.0. It's free to use provided the work you do is freely available!

For proprietary use, please [contact me](mailto:kzm@zkat.tech?subject=cacache%20license), or just [sponsor me on GitHub](https://github.com/users/zkat/sponsorship) under the appropriate tier to [acquire a proprietary-use license](LICENSE-PATRON.md)! This funding model helps me make my work sustainable and compensates me for the work it took to write this crate!
