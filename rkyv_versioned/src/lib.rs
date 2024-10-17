use core::{error::Error, fmt};
use rkyv::api::high::HighSerializer;
use rkyv::ser::allocator::ArenaHandle;
use rkyv::util::AlignedVec;
use rkyv::with::Inline;
use rkyv::{Archive, Serialize};

// Re-export the derive macro
pub use rkyv_versioned_derive::VersionedArchiveContainer;

#[derive(Debug)]
pub struct UnexpectedTypeError(u32, u32);
impl Error for UnexpectedTypeError {}
impl fmt::Display for UnexpectedTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expected type_id {}, got {}", self.0, self.1)
    }
}

#[derive(Debug)]
pub struct UnsupportedVersionError(u32);
impl Error for UnsupportedVersionError {}
impl fmt::Display for UnsupportedVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported version {}", self.0)
    }
}

// This is only referenced in the macro and it's not picked up here.
#[allow(dead_code)]
#[derive(Debug, Clone, Archive, Serialize)]
struct TaggedVersionedContainer<'a, T: Archive>(u32, u32, #[rkyv(with = Inline)] &'a T);

#[derive(Debug, Clone, Archive, Serialize)]
struct TaggedVersionedContainerHeaderOnly(u32, u32);

pub trait VersionedContainer: Archive {
    const ARCHIVE_TYPE_ID: u32;
    fn is_valid_version_id(version: u32) -> bool;
    fn get_entry_version_id(&self) -> u32;

    fn get_type_and_version_from_tagged_bytes(
        buf: &[u8],
    ) -> Result<(u32, u32), rkyv::rancor::Error>;
    fn get_ref_from_tagged_bytes(buf: &[u8]) -> Result<&Self::Archived, rkyv::rancor::Error>;
    fn to_tagged_bytes(item: &Self) -> Result<AlignedVec, rkyv::rancor::Error>
    where
        Self: for<'a> Serialize<HighSerializer<AlignedVec, ArenaHandle<'a>, rkyv::rancor::Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rkyv::Deserialize;

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
    enum TestContainer<'a> {
        //    #[rkyv_util_version(1)]
        V1(#[rkyv(with=Inline)] &'a TestStructV1),
        //    #[rkyv_util_version(2)]
        V2(#[rkyv(with=Inline)] &'a TestStructV2),
    }

    #[test]
    fn test_versioned_container() {
        let v1 = TestStructV1 {
            a: 1,
            b: 2,
            c: "YEET".to_owned(),
        };
        let v1_container = TestContainer::V1(&v1);

        let tswv_container_bytes: AlignedVec =
            TestContainer::to_tagged_bytes(&v1_container).unwrap();
        assert_eq!(
            TestContainer::get_type_and_version_from_tagged_bytes(&tswv_container_bytes).unwrap(),
            (
                TestContainer::ARCHIVE_TYPE_ID,
                v1_container.get_entry_version_id()
            )
        );

        let twsv_ref = TestContainer::get_ref_from_tagged_bytes(&tswv_container_bytes).unwrap();

        match twsv_ref {
            ArchivedTestContainer::V1(v1_ref) => {
                assert_eq!(v1_ref.a, 1);
                assert_eq!(v1_ref.b, 2);
                assert_eq!(v1_ref.c, "YEET");
            }
            _ => panic!("Expected V1"),
        }

        let v2 = TestStructV2 {
            a: 100,
            b: 200,
            c: 300,
            d: "SKEET".to_owned(),
        };
        let v2_container = TestContainer::V2(&v2);
        let tswv_container_bytes: AlignedVec =
            TestContainer::to_tagged_bytes(&v2_container).unwrap();
        assert_eq!(
            TestContainer::get_type_and_version_from_tagged_bytes(&tswv_container_bytes).unwrap(),
            (
                TestContainer::ARCHIVE_TYPE_ID,
                v2_container.get_entry_version_id()
            )
        );
        let twsv_ref = TestContainer::get_ref_from_tagged_bytes(&tswv_container_bytes).unwrap();

        match twsv_ref {
            ArchivedTestContainer::V2(v2_ref) => {
                assert_eq!(v2_ref.a, 100);
                assert_eq!(v2_ref.b, 200);
                assert_eq!(v2_ref.c, 300);
                assert_eq!(v2_ref.d, "SKEET");
            }
            _ => panic!("Expected V2"),
        }

        // Generate invalid type id
        const EXPECTED_TYPE_ID: u32 = const_crc32::crc32("TestContainer".as_bytes());
        const MUNGED_TYPE_ID: u32 = 0x01010101;

        let mut invalid_type_bytes = tswv_container_bytes.clone();
        invalid_type_bytes[0] = 0x01;
        invalid_type_bytes[1] = 0x01;
        invalid_type_bytes[2] = 0x01;
        invalid_type_bytes[3] = 0x01;

        let invalid_type_result = TestContainer::get_ref_from_tagged_bytes(&invalid_type_bytes);
        assert!(invalid_type_result.is_err());
        assert_eq!(
            invalid_type_result.err().unwrap().to_string(),
            format!(
                "Expected type_id {}, got {}",
                EXPECTED_TYPE_ID, MUNGED_TYPE_ID
            )
        );

        // Generate invalid version id
        let mut invalid_ver_bytes = tswv_container_bytes.clone();
        invalid_ver_bytes[4] = 9;

        let invalid_ver_result = TestContainer::get_ref_from_tagged_bytes(&invalid_ver_bytes);
        assert!(invalid_ver_result.is_err());
        assert_eq!(
            invalid_ver_result.err().unwrap().to_string(),
            "Unsupported version 9"
        );
    }
}
