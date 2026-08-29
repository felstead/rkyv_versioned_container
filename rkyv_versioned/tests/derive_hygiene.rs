//! The derive must compile for a user who imports only what they use, without glob-importing
//! the crate and without `VersionedContainer` in scope.

use rkyv::{Archive, Deserialize, Serialize};
use rkyv_versioned::VersionedArchiveContainer;

#[derive(Debug, Archive, Serialize, Deserialize)]
struct StructV1 {
    a: u32,
}

#[derive(Debug, Archive, Serialize, Deserialize)]
struct StructV2 {
    a: u64,
}

#[derive(Debug, Archive, Serialize, Deserialize, VersionedArchiveContainer)]
enum Container {
    V1(StructV1),
    V2(StructV2),
}

#[test]
fn derived_container_round_trips_without_glob_import() {
    use rkyv_versioned::VersionedContainer;

    let bytes = rkyv_versioned::to_tagged_bytes(&Container::V2(StructV2 { a: 7 })).unwrap();
    let (type_id, version_id) =
        rkyv_versioned::get_type_and_version_from_tagged_bytes(&bytes).unwrap();

    assert_eq!(type_id, Container::ARCHIVE_TYPE_ID);
    assert_eq!(version_id, Container::NEWEST_VERSION_ID);
}
