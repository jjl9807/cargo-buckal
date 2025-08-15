use serde::ser::{Serialize, SerializeStruct, SerializeTupleStruct, Serializer};
use serde_derive::Serialize;
use std::collections::{BTreeMap as Map, BTreeSet as Set};

#[derive(Serialize)]
#[serde(untagged)]
pub enum Rule {
    Load(Load),
    CargoRustLibrary(CargoRustLibrary),
    CargoRustBinary(CargoRustBinary),
    BuildscriptRun(BuildscriptRun),
}

pub struct Load {
    pub bzl: String,
    pub items: Set<String>,
}

#[derive(Serialize, Default)]
#[serde(rename = "cargo.rust_library")]
pub struct CargoRustLibrary {
    pub name: String,
    pub srcs: Glob,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub crate_root: String,
    pub edition: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub env: Map<String, String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub features: Set<String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub rustc_flags: Set<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proc_macro: Option<bool>,
    pub visibility: Set<String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub deps: Set<String>,
}

#[derive(Serialize, Default)]
#[serde(rename = "cargo.rust_binary")]
pub struct CargoRustBinary {
    pub name: String,
    pub srcs: Glob,
    #[serde(rename = "crate")]
    pub crate_name: String,
    pub crate_root: String,
    pub edition: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub env: Map<String, String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub features: Set<String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub rustc_flags: Set<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proc_macro: Option<bool>,
    pub visibility: Set<String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub deps: Set<String>,
}

#[derive(Serialize, Default)]
#[serde(rename = "buildscript_run")]
pub struct BuildscriptRun {
    pub name: String,
    pub package_name: String,
    pub buildscript_rule: String,
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub env: Map<String, String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub features: Set<String>,
    pub version: String,
    pub local_manifest_dir: String,
}

#[derive(Default)]
pub struct Glob {
    pub include: Set<String>,
    pub exclude: Set<String>,
}

impl Serialize for Load {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_tuple_struct("load", 0)?;
        s.serialize_field(&self.bzl)?;
        for item in &self.items {
            s.serialize_field(item)?;
        }
        s.end()
    }
}

impl Serialize for Glob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.exclude.is_empty() {
            serializer.serialize_newtype_struct("glob", &self.include)
        } else {
            let mut s = serializer.serialize_struct("glob", 2)?;
            s.serialize_field("include", &self.include)?;
            s.serialize_field("exclude", &self.exclude)?;
            s.end()
        }
    }
}
