# rkyv_versioned_derive

Derive macro for [rkyv_versioned](https://crates.io/crates/rkyv_versioned) containers. This crate is
not used directly: depend on `rkyv_versioned`, which re-exports the derive.

```rust
#[derive(Debug, Archive, Serialize, Deserialize, VersionedArchiveContainer)]
enum EntryContainer {
    V1(EntryV1),
    V2(EntryV2),
}
```

`#[derive(VersionedArchiveContainer)]` implements `rkyv_versioned::VersionedContainer`, giving the
container a type ID hashed from its name, a version ID per variant, and the constants error messages
report. It accepts `#[rkyv_versioned(archive_type_name = "...")]` to pin the string the type ID is
hashed from.

See the [rkyv_versioned documentation](https://docs.rs/rkyv_versioned) for usage.

## License

MIT. See [LICENSE](LICENSE).
