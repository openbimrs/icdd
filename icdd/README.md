# icdd

Short-name package for ISO 21597 Information Container for linked Document
Delivery.

This package is a **pure re-export** of
[`openbim-icdd`](https://crates.io/crates/openbim-icdd). It defines nothing of
its own, so both package names expose exactly the same types and behavior.

```toml
# Choose either package; do not add both directly.
icdd = "0.2"
# openbim-icdd = "0.2"
```

The dependency is pinned to the exact canonical version, and the repository gate
rejects implementation or independent type definitions in this package. See
[`openbim-icdd`](https://crates.io/crates/openbim-icdd) for exact capability
status.

## License

MIT
