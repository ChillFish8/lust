use std::hash::Hash;

pub fn crc_hash<H: Hash>(v: H) -> u32 {
    let mut hasher = crc32fast::Hasher::default();
    v.hash(&mut hasher);
    hasher.finalize()
}