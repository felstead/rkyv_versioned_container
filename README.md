# rkyv_versioned

`rkyv_versioned` is a Rust library that provides an ergonomic versioned container for [rkyv](https://github.com/rkyv/rkyv) archives. This allows for both backwards and forwards compatibility for when the structures of the archives change over time.

## Features

- **Versioned Containers**: Easily manage different versions of your data `rkyv` structures.
- **Backwards and Forwards Compatibility**: Access older versions of your serialized `rkyv` data without issues, and be able to identify newer versions.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
rkyv_versioned = "0.1.0"
```

## Usage

To provide backwards and forwards compatibility between structures formatted by `rkyv`, we follow these steps:
- We should provide implementations of all "known" versions of an `rkyv` structure in our code (see `TestStructV1` and `TestStructV2` in the example below)
- We wrap these versions in an enum describing all of the different versions, and use the `#[derive(VersionedArchiveContainer)]` macro on that enum in addition to your usual `#[derive(Archive, Serialize, Deserialize)]` definitions for an `rkyv` type, see `TestVersionedContainer` in the example below.

However, there are some important rules to abide by:
- **The layout/structure of the `rkyv` implementations MUST NOT CHANGE between versions of the code** - if you make changes, it is important to declare a new type and add it to our versioned container. This is because we will try to deserialize/access the data using the implementation in the current code, so if we serialize `TestStructV1` with one layout and then change it later, it may not be able to be read correctly.  Instead, try declaring `TestStructV2` and add it to our versioned container.
- **The versioned container's enum order MUST NOT CHANGE** - the IDs of each variant are based on their order, so it is important to keep this consistent and **only add new variants to the end of the struct**.

An example:

```rust
use rkyv::{Archive, Serialize, Deserialize};
use rkyv_versioned_container::*;

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
struct TestStructV1 {
    pub a: u32,
    pub b: u32,
    pub c: String,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
struct TestStructV2 {
    pub a: u64,
    pub b: u64,
    pub c: u64,
    pub d: String,
}

#[derive(Debug, Clone, Archive, Serialize, Deserialize, VersionedArchiveContainer)]
enum TestVersionedContainer<'a> {
    V1(#[rkyv(with=Inline)] &'a TestStructV1),
    V2(#[rkyv(with=Inline)] &'a TestStructV2),
}

fn main() {
    // Serialize a v1 into a versioned container byte stream
    let v1 = TestStructV1 {
        a: 1,
        b: 2,
        c: "YEET".to_owned(),
    };

    // Create our versioned container to store our v1 data
    let container = TestVersionedContainer::V1(&v1);

    // This byte stream contains extra metadata allowing you to identify the type and version before
    // attempting to access it
    let tswv_container_bytes: AlignedVec = TestVersionedContainer::to_tagged_bytes(&container).unwrap();

    // Imagine now that you're reading this byte stream from a file or network - it is _probably_ a
    // TestContainer::V1, but you can't be sure, it _could_ be a TestContainer::V2 (which would
    // be fine) or, if we're older version of the code against newer data, a TestContainer::V3.  Or
    // maybe it's not even a TestContainer at all. With the tagged container, we can validate
    // beforehand, or have logic to handle different structures or versions.
    let (type_id, version_id) =
        TestVersionedContainer::get_type_and_version_from_tagged_bytes(&tswv_container_bytes).unwrap();
    assert_eq!(type_id, TestVersionedContainer::ARCHIVE_TYPE_ID);
    assert_eq!(version_id, container.get_entry_version_id());

    // You can now more confidently access the data using zero-copy rkyv primitives
    let twsv_ref: &ArchivedTestVersionedContainer<'_> =
        TestVersionedContainer::access_from_tagged_bytes(&tswv_container_bytes).unwrap();
    match twsv_ref {
        ArchivedTestVersionedContainer::V1(v1_ref) => {
            assert_eq!(v1_ref.a, 1);
            assert_eq!(v1_ref.b, 2);
            assert_eq!(v1_ref.c, "YEET");
        }
        _ => panic!("Expected V1"),
    }
}
```

## Implementation
The `#[derive(VersionedArchiveContainer)]` will implement the `VersionedContainer` trait on the enum:

```rust
pub trait VersionedContainer: Archive {
    const ARCHIVE_TYPE_ID: u32;
    fn is_valid_version_id(version: u32) -> bool;
    fn get_entry_version_id(&self) -> u32;

    fn get_type_and_version_from_tagged_bytes(
        buf: &[u8],
    ) -> Result<(u32, u32), rkyv::rancor::Error>;
    fn access_from_tagged_bytes(buf: &[u8]) -> Result<&Self::Archived, rkyv::rancor::Error>;
    fn to_tagged_bytes(item: &Self) -> Result<AlignedVec, rkyv::rancor::Error>
    where
        Self: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>>;
}
```

This generated code will include a (mostly) unique `u32` ID for the type in `ARCHIVE_TYPE_ID` (based on the crc32 of the container type name, e.g. `crc32(TestVersionedContainer)`) and it will generate incrementing IDs for each variant of its containing struct, e.g. `V1` has a version ID of `0`, `V2` has a version ID of `1` and so on.

When the data is serialized using `to_tagged_bytes` it is serialized as a tuple of `(archive_type_id, version_id, Self::Archived)`.  The layout of `rkyv` tuples allow us to "peek" at the `archive_type_id` and `version_id` at the head of the byte stream without doing any further checking/deserialization of the enum type - this allows for guarding against unknown/unsupported data.


## Documentation

For detailed documentation, please visit [docs.rs](https://docs.rs/rkyv_versioned).

## Contributing

We welcome contributions! Please see our [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Acknowledgements

Special thanks to the contributors of the [rkyv](https://github.com/rkyv/rkyv) project for their foundational work.
