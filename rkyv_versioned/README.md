# rkyv_versioned

Versioned containers for [rkyv](https://github.com/rkyv/rkyv) archives, so data written by an older
or newer build of your code can still be identified and read. Serialized data gets a small header
holding a type ID and a version ID, which a reader checks before touching the data itself.

```toml
[dependencies]
rkyv_versioned = "0.2"
```

Declare every version of a structure as its own type, wrap them in an enum deriving
`VersionedArchiveContainer`, and serialize through `to_tagged_bytes`:

```rust
#[derive(Debug, Archive, Serialize, Deserialize, VersionedArchiveContainer)]
enum EntryContainer {
    V1(EntryV1),
    V2(EntryV2),
}

let bytes = to_tagged_bytes(&EntryContainer::V2(entry))?;
let archived = access_from_tagged_bytes::<EntryContainer>(&bytes)?;
```

Reading data written by a newer build fails with a clear error rather than misreading it:

```text
EntryContainer: unsupported version 3 (newest known: 1)
```

Accessors on the archived container keep read sites free of version-specific code, and
`VersionedUpgrade` converts any version to an owned value of the newest type for rewrite passes.

See the [crate documentation](https://docs.rs/rkyv_versioned) for both patterns, the buffer
contract, and the container attributes, or the
[repository](https://github.com/felstead/rkyv_versioned_container) for the full README.

## License

MIT. See [LICENSE](LICENSE).
