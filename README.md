# rkyv_versioned

[![crates.io](https://img.shields.io/crates/v/rkyv_versioned.svg)](https://crates.io/crates/rkyv_versioned)
[![docs.rs](https://docs.rs/rkyv_versioned/badge.svg)](https://docs.rs/rkyv_versioned)

`rkyv_versioned` provides versioned containers for [rkyv](https://github.com/rkyv/rkyv) archives, so
data written by an older or newer build of your code can still be identified and read. Serialized
data gets a small header holding a type ID and a version ID, which a reader checks before touching
the data itself.

```toml
[dependencies]
rkyv_versioned = "0.2"
```

## How it works

Declare every version of a structure as its own type, then wrap them in an enum that derives
`VersionedArchiveContainer` alongside rkyv's usual derives:

```rust
use rkyv::{Archive, Serialize, Deserialize};
use rkyv_versioned::*;

#[derive(Debug, Archive, Serialize, Deserialize)]
struct EntryV1 {
    pub revision: u32,
    pub name: String,
}

#[derive(Debug, Archive, Serialize, Deserialize)]
struct EntryV2 {
    pub revision: u64,
    pub name: String,
    pub retired: bool,
}

#[derive(Debug, Archive, Serialize, Deserialize, VersionedArchiveContainer)]
enum EntryContainer {
    V1(EntryV1),
    V2(EntryV2),
}
```

`to_tagged_bytes` serializes a container, and `access_from_tagged_bytes` validates the header before
handing back the archived data:

```rust
let bytes = to_tagged_bytes(&EntryContainer::V2(EntryV2 {
    revision: 7,
    name: "main".to_owned(),
    retired: false,
}))?;

// The header can be read without touching the data, to decide whether this is yours to read
let (type_id, version_id) = get_type_and_version_from_tagged_bytes(&bytes)?;
assert_eq!(type_id, EntryContainer::ARCHIVE_TYPE_ID);

let entry = access_from_tagged_bytes::<EntryContainer>(&bytes)?;
```

Reading data written by a *newer* build fails with a clear error rather than misreading it:

```text
EntryContainer: unsupported version 3 (newest known: 1)
```

## Two rules

- **The layout of each version's type must never change.** Data already written was serialized
  against the layout in the code at the time, so changing a type in place makes existing data
  unreadable. Declare a new type and add it to the container instead.
- **The container's variant order must never change.** Version IDs come from variant position, so
  add new variants only at the end.

## Reading across versions

Destructuring the container at each read site, `let EntryContainer::V1(v1) = &container`, ties every
one of those sites to a version, so adding a `V3` means editing all of them. Put the version
knowledge in accessors on the archived container instead:

```rust
impl ArchivedEntryContainer {
    pub fn revision(&self) -> u64 {
        match self {
            // Note we need to upcast the revision from u32 to u64 for V1 for consistency
            ArchivedEntryContainer::V1(v1) => v1.revision.to_native() as u64,
            // Latest is already u64, so no conversion needed
            ArchivedEntryContainer::V2(v2) => v2.revision.to_native(),
        }
    }

    // Added in V2, so older data answers with a documented default
    pub fn retired(&self) -> bool {
        match self {
            // Older versions did not have the `retired` field, so we return the default
            ArchivedEntryContainer::V1(_) => false,
            // Otherwise return the canonical value from the current version
            ArchivedEntryContainer::V2(v2) => v2.retired,
        }
    }
}
```

Reading code calls `entry.revision()` and works against either version, with no conversion and no
allocation, since accessors read straight out of the mapped bytes. Adding a `V3` makes each
accessor's match non-exhaustive, so the compiler points at the accessors to extend and leaves call
sites alone.

## Upgrading to the latest version

Code that needs an *owned* value of the newest type, such as a pass that rewrites stored records,
implements `VersionedUpgrade` on the archived container:

```rust
impl VersionedUpgrade for ArchivedEntryContainer {
    type Latest = EntryV2;

    fn upgrade(&self) -> Result<MaybeUpgraded<'_, EntryV2>, RkyvVersionedError> {
        match self {
            ArchivedEntryContainer::V1(v1) => Ok(MaybeUpgraded::Upgraded(EntryV2 {
                revision: v1.revision.to_native() as u64,
                name: v1.name.to_string(),
                retired: false,
            })),
            ArchivedEntryContainer::V2(v2) => Ok(MaybeUpgraded::Current(v2)),
        }
    }
}
```

`MaybeUpgraded` works like `std::borrow::Cow`: data already at the latest version stays zero-copy in
the `Current` arm, and only older data pays for a conversion. `into_owned()` produces an owned value
from either arm. An arm that deliberately refuses to convert older data returns
`RkyvVersionedError::UpgradeNotSupported`.

## Container attributes

```rust
#[derive(Debug, Archive, Serialize, Deserialize, VersionedArchiveContainer)]
#[rkyv_versioned(archive_type_name = "myapp::LedgerEntry")]
enum LedgerEntryContainer {
    V1(EntryV1),
}
```

`archive_type_name` pins the string that the type ID is hashed from, instead of the container's
identifier. Use it when two containers readable from the same store would otherwise share an
identifier, or to keep the ID stable across a future rename. Setting or changing it changes the
stored type ID, so pick it before writing data.

## Buffer contract

A tagged byte array must be handed back to the accessors exactly as it came out of
`to_tagged_bytes`. rkyv locates the archived root from the *end* of the buffer, so a buffer with
trailing bytes reads the header from the wrong offset. If records are stored in fixed-size pages,
keep the serialized length alongside each record and slice to it before reading, because the length
cannot be recovered from the tagged bytes. Alignment follows rkyv's usual rules for the archived
type.

## Serialized form

`to_tagged_bytes` serializes a `TaggedVersionedStruct`:

```rust
pub struct TaggedVersionedStruct<'a, T: Archive> {
    pub type_id: u32,
    pub version_id: u32,
    #[rkyv(with = InlineAsBox)]
    pub inner: &'a T,
}
```

`type_id` is a CRC32 of the container's name, and `version_id` is the position of the variant, so
`V1` is `0`, `V2` is `1`, and so on. Accessing this with `T` as the unit type reads the two header
fields without deserializing `inner`, which is how the header peek works. The wrapper costs 12 bytes
per record.

## Documentation

Full documentation, including the error type and the unchecked accessor's safety contract, is on
[docs.rs](https://docs.rs/rkyv_versioned).

## Development

```bash
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt
dprint fmt
```

`dprint fmt` formats the Markdown files, wrapping prose at 100 columns so hand-edited documents stay
consistent. Install it with `cargo install dprint`, and use `dprint check` to verify without
writing. The plugin version is pinned in [dprint.json](dprint.json), so formatting does not change
underfoot.

## License

MIT. See [LICENSE](LICENSE).
