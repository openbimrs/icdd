# icdd

ISO 21597 ICDD (Information Container for linked Document Delivery) for Rust.

This crate is a **pure re-export** of [`openbim-icdd`](https://crates.io/crates/openbim-icdd). It
defines nothing of its own — it exists so the standard is reachable under the
short name practitioners actually use, while there remains exactly one
definition of every type.

```toml
icdd = "0.1"
# identical to:
openbim-icdd = "0.1"
```

Use whichever name reads better in your project. Do not depend on both.

## Status

**Reserved.** See [`openbim-icdd`](https://crates.io/crates/openbim-icdd) for what is implemented.

## Part of nehirde

A pure-Rust IFC and openBIM toolchain: <https://github.com/GeneralPawz/nehirde>

## License

MIT
