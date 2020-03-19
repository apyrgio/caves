# Caves

A collection of embedded, thread-safe key-value stores (kvs) in Rust.

[![CI](https://github.com/apyrgio/caves/workflows/CI/badge.svg?branch=master&event=schedule)](https://github.com/apyrgio/caves/actions?query=event%3Aschedule+branch%3Amaster)
[![Crates.io](https://img.shields.io/crates/v/caves.svg)](https://crates.io/crates/caves)
[![Docs.rs](https://docs.rs/caves/badge.svg)](https://docs.rs/caves)

## Overview

The `caves` crate provides a selection of key-value stores with the
following features:

* [Embedded]
* Thread-safe
* Simple API; get/set/delete a key
* Dev-friendly

You can find more info on the rationale behind this crate on
https://docs.rs/caves.

## Usage

```rust
use caves::errors::Error;
use caves::{MemoryCave, Cave};

// Initialize a MemoryCave object.
let b = MemoryCave::new();

// Create a new key with an empty value.
b.set("key", b"");

// Override the key's value.
b.set("key", b"value");

// Retrieve the contents of the key.
let res = b.get("key");
assert_eq!(res.unwrap(), b"value");

// Delete the key.
b.delete("key");

// Subsequent attempts to retrieve the contents of the key should return an
// error.
let res = b.get("key");
assert_eq!(res, Err(Error::NotFound("key".to_string())));
```

## Documentation

You can read the latest docs in https://docs.rs/caves.

## Development

Clone this repo, make changes to the code and ensure that the tests pass:

```
$ cargo test
```

Finally, read the [`NOTICE.md`] file for the legal status of the project.

## Legal

Licensed under MPL-2.0. Please read the [`NOTICE.md`] and [`LICENSE`] files for
the full copyright and license information.

[Embedded]: https://en.wikipedia.org/wiki/Embedded_database
[`NOTICE.md`]: /NOTICE.md
[`LICENSE`]: /LICENSE
