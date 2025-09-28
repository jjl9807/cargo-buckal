use std::collections::{BTreeMap, HashMap};

use anyhow::{Error, Result, anyhow};
use bincode::config::Configuration;
use blake3::hash;
use cargo_metadata::{Node, PackageId};
use serde::{Deserialize, Serialize};

use crate::utils::{UnwrapOrExit, get_cache_dir, get_cache_path};

type Fingerprint = [u8; 32];

pub trait BuckalExt {
    fn fingerprint(&self) -> Fingerprint;
}

impl BuckalExt for Node {
    fn fingerprint(&self) -> Fingerprint {
        let encoded = bincode::serde::encode_to_vec(self, bincode::config::standard())
            .expect("Serialization failed");
        hash(&encoded).into()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuckalCache {
    fingerprints: HashMap<PackageId, Fingerprint>,
    version: u32,
}

impl BuckalCache {
    pub fn new(resolve: &HashMap<PackageId, Node>) -> Self {
        let fingerprints = resolve
            .iter()
            .map(|(id, node)| (id.clone(), node.fingerprint()))
            .collect();
        Self {
            fingerprints,
            version: 1,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            fingerprints: HashMap::new(),
            version: 1,
        }
    }

    pub fn load() -> Result<Self, Error> {
        let cache_path = get_cache_path().unwrap_or_exit_ctx("failed to get cache path");
        if !cache_path.exists() {
            return Err(anyhow!("Cache file does not exist"));
        }
        if let Ok(mut cache_reader) = std::fs::File::open(&cache_path)
            && let Ok(cache) = bincode::serde::decode_from_std_read::<
                BuckalCache,
                Configuration,
                std::fs::File,
            >(&mut cache_reader, bincode::config::standard())
        {
            return Ok(cache);
        }
        Err(anyhow!("Cache version mismatch"))
    }

    pub fn save(&self) {
        let cache_path = get_cache_path().unwrap_or_exit_ctx("failed to get cache path");
        let cache_dir = get_cache_dir().unwrap_or_exit_ctx("failed to get cache directory");
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
        }

        if let Ok(mut cache_writer) = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&cache_path)
        {
            let _ = bincode::serde::encode_into_std_write(
                self,
                &mut cache_writer,
                bincode::config::standard(),
            );
        } else {
            panic!("Failed to open cache file for writing");
        }
    }

    pub fn diff(&self, other: &BuckalCache) -> BuckalChange {
        let mut _diff = BuckalChange::default();
        for (id, fp) in &self.fingerprints {
            if let Some(other_fp) = other.fingerprints.get(id) {
                if fp != other_fp {
                    _diff.changes.insert(id.to_owned(), ChangeType::Changed);
                }
            } else {
                // new package added in self
                _diff.changes.insert(id.clone(), ChangeType::Added);
            }
        }
        for id in other.fingerprints.keys() {
            if !self.fingerprints.contains_key(id) {
                // redundant package removed in self
                _diff.changes.insert(id.clone(), ChangeType::Removed);
            }
        }
        _diff
    }
}

#[derive(Debug, Default)]
pub struct BuckalChange {
    pub changes: BTreeMap<PackageId, ChangeType>,
}

#[derive(Debug)]
pub enum ChangeType {
    Added,
    Removed,
    Changed,
}
