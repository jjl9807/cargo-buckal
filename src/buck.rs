use cargo_metadata::camino::Utf8PathBuf;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};
use pyo3_ffi::c_str;
use serde::ser::{Serialize, SerializeStruct, SerializeTupleStruct, Serializer};
use serde_derive::Serialize;
use std::collections::{BTreeMap as Map, BTreeSet as Set};
use std::ffi::CString;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Rule {
    Load(Load),
    CargoRustLibrary(CargoRustLibrary),
    CargoRustBinary(CargoRustBinary),
    BuildscriptRun(BuildscriptRun),
}

pub trait CargoRule {
    fn deps_mut(&mut self) -> &mut Set<String>;
    fn rustc_flags_mut(&mut self) -> &mut Set<String>;
    fn env_mut(&mut self) -> &mut Map<String, String>;
}

#[derive(Debug)]
pub struct Load {
    pub bzl: String,
    pub items: Set<String>,
}

#[derive(Serialize, Default, Debug)]
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

#[derive(Serialize, Default, Debug)]
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

#[derive(Serialize, Default, Debug)]
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

#[derive(Default, Debug)]
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

impl Glob {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let include_vec: Vec<String> = kwargs
            .get_item("include")
            .expect("Expected 'include' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let include: Set<String> = include_vec.into_iter().collect();
        let exclude_vec: Vec<String> = kwargs
            .get_item("exclude")
            .expect("Expected 'exclude' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let exclude: Set<String> = exclude_vec.into_iter().collect();
        Ok(Glob { include, exclude })
    }

    fn patch_from(&mut self, other: &Glob) {
        // Patch include set
        let to_add: Vec<_> = other.include.difference(&self.include).cloned().collect();
        self.include.extend(to_add);
        // Patch exclude set
        let to_add: Vec<_> = other.exclude.difference(&self.exclude).cloned().collect();
        self.exclude.extend(to_add);
    }
}

impl CargoRule for CargoRustLibrary {
    fn deps_mut(&mut self) -> &mut Set<String> {
        &mut self.deps
    }

    fn rustc_flags_mut(&mut self) -> &mut Set<String> {
        &mut self.rustc_flags
    }

    fn env_mut(&mut self) -> &mut Map<String, String> {
        &mut self.env
    }
}

impl CargoRule for CargoRustBinary {
    fn deps_mut(&mut self) -> &mut Set<String> {
        &mut self.deps
    }

    fn rustc_flags_mut(&mut self) -> &mut Set<String> {
        &mut self.rustc_flags
    }

    fn env_mut(&mut self) -> &mut Map<String, String> {
        &mut self.env
    }
}

impl CargoRustLibrary {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = kwargs
            .get_item("name")
            .expect("Expected 'name' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let src_kwargs_tuple_binding = kwargs
            .get_item("srcs")
            .expect("Expected 'srcs' argument")
            .unwrap();
        let src_kwargs_tuple = src_kwargs_tuple_binding.downcast::<PyTuple>().unwrap();
        let src_func_binding = src_kwargs_tuple.get_item(0).unwrap();
        let src_func = src_func_binding.downcast::<PyString>().unwrap();
        let src_kwargs_binding = src_kwargs_tuple.get_item(1).unwrap();
        let src_kwargs = src_kwargs_binding.downcast::<PyDict>().unwrap();
        assert!(src_func.extract::<&str>().unwrap() == "glob");
        let srcs = Glob::from_py_dict(src_kwargs)?;
        let crate_name: String = kwargs
            .get_item("crate")
            .expect("Expected 'crate' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let crate_root: String = kwargs
            .get_item("crate_root")
            .expect("Expected 'crate_root' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let edition: String = kwargs
            .get_item("edition")
            .expect("Expected 'edition' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let env: Map<String, String> = kwargs
            .get_item("env")
            .expect("Expected 'env' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let features_vec: Vec<String> = kwargs
            .get_item("features")
            .expect("Expected 'features' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let features: Set<String> = features_vec.into_iter().collect();
        let rustc_flags_vec: Vec<String> = kwargs
            .get_item("rustc_flags")
            .expect("Expected 'rustc_flags' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let rustc_flags: Set<String> = rustc_flags_vec.into_iter().collect();
        let proc_macro: Option<bool> = kwargs
            .get_item("proc_macro")
            .expect("Expected 'proc_macro' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let visibility_vec: Vec<String> = kwargs
            .get_item("visibility")
            .expect("Expected 'visibility' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let visibility: Set<String> = visibility_vec.into_iter().collect();
        let deps_vec: Vec<String> = kwargs
            .get_item("deps")
            .expect("Expected 'deps' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let deps: Set<String> = deps_vec.into_iter().collect();
        Ok(CargoRustLibrary {
            name,
            srcs,
            crate_name,
            crate_root,
            edition,
            env,
            features,
            rustc_flags,
            proc_macro,
            visibility,
            deps,
        })
    }

    fn patch_from(&mut self, other: &CargoRustLibrary) {
        // Patch srcs glob
        self.srcs.patch_from(&other.srcs);
        // Patch env map
        for (k, v) in &other.env {
            self.env.entry(k.clone()).or_insert_with(|| v.clone());
        }
        // Patch features set
        let to_add: Vec<_> = other.features.difference(&self.features).cloned().collect();
        self.features.extend(to_add);
        // Patch rustc_flags set
        let to_add: Vec<_> = other
            .rustc_flags
            .difference(&self.rustc_flags)
            .cloned()
            .collect();
        self.rustc_flags.extend(to_add);
        // Patch visibility set
        let to_add: Vec<_> = other
            .visibility
            .difference(&self.visibility)
            .cloned()
            .collect();
        self.visibility.extend(to_add);
        // Patch deps set
        let to_add: Vec<_> = other.deps.difference(&self.deps).cloned().collect();
        self.deps.extend(to_add);
    }
}

impl CargoRustBinary {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = kwargs
            .get_item("name")
            .expect("Expected 'name' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let src_kwargs_tuple_binding = kwargs
            .get_item("srcs")
            .expect("Expected 'srcs' argument")
            .unwrap();
        let src_kwargs_tuple = src_kwargs_tuple_binding.downcast::<PyTuple>().unwrap();
        let src_func_binding = src_kwargs_tuple.get_item(0).unwrap();
        let src_func = src_func_binding.downcast::<PyString>().unwrap();
        let src_kwargs_binding = src_kwargs_tuple.get_item(1).unwrap();
        let src_kwargs = src_kwargs_binding.downcast::<PyDict>().unwrap();
        assert!(src_func.extract::<&str>().unwrap() == "glob");
        let srcs = Glob::from_py_dict(src_kwargs)?;
        let crate_name: String = kwargs
            .get_item("crate")
            .expect("Expected 'crate' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let crate_root: String = kwargs
            .get_item("crate_root")
            .expect("Expected 'crate_root' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let edition: String = kwargs
            .get_item("edition")
            .expect("Expected 'edition' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let env: Map<String, String> = kwargs
            .get_item("env")
            .expect("Expected 'env' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let features_vec: Vec<String> = kwargs
            .get_item("features")
            .expect("Expected 'features' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let features: Set<String> = features_vec.into_iter().collect();
        let rustc_flags_vec: Vec<String> = kwargs
            .get_item("rustc_flags")
            .expect("Expected 'rustc_flags' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let rustc_flags: Set<String> = rustc_flags_vec.into_iter().collect();
        let proc_macro: Option<bool> = kwargs
            .get_item("proc_macro")
            .expect("Expected 'proc_macro' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let visibility_vec: Vec<String> = kwargs
            .get_item("visibility")
            .expect("Expected 'visibility' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let visibility: Set<String> = visibility_vec.into_iter().collect();
        let deps_vec: Vec<String> = kwargs
            .get_item("deps")
            .expect("Expected 'deps' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let deps: Set<String> = deps_vec.into_iter().collect();
        Ok(CargoRustBinary {
            name,
            srcs,
            crate_name,
            crate_root,
            edition,
            env,
            features,
            rustc_flags,
            proc_macro,
            visibility,
            deps,
        })
    }

    fn patch_from(&mut self, other: &CargoRustBinary) {
        // Patch srcs glob
        self.srcs.patch_from(&other.srcs);
        // Patch env map
        for (k, v) in &other.env {
            self.env.entry(k.clone()).or_insert_with(|| v.clone());
        }
        // Patch features set
        let to_add: Vec<_> = other.features.difference(&self.features).cloned().collect();
        self.features.extend(to_add);
        // Patch rustc_flags set
        let to_add: Vec<_> = other
            .rustc_flags
            .difference(&self.rustc_flags)
            .cloned()
            .collect();
        self.rustc_flags.extend(to_add);
        // Patch visibility set
        let to_add: Vec<_> = other
            .visibility
            .difference(&self.visibility)
            .cloned()
            .collect();
        self.visibility.extend(to_add);
        // Patch deps set
        let to_add: Vec<_> = other.deps.difference(&self.deps).cloned().collect();
        self.deps.extend(to_add);
    }
}

impl BuildscriptRun {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = kwargs
            .get_item("name")
            .expect("Expected 'name' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let package_name: String = kwargs
            .get_item("package_name")
            .expect("Expected 'package_name' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let buildscript_rule: String = kwargs
            .get_item("buildscript_rule")
            .expect("Expected 'buildscript_rule' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let env: Map<String, String> = kwargs
            .get_item("env")
            .expect("Expected 'env' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let features_vec: Vec<String> = kwargs
            .get_item("features")
            .expect("Expected 'features' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let features: Set<String> = features_vec.into_iter().collect();
        let version: String = kwargs
            .get_item("version")
            .expect("Expected 'version' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let local_manifest_dir: String = kwargs
            .get_item("local_manifest_dir")
            .expect("Expected 'local_manifest_dir' argument")
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        Ok(BuildscriptRun {
            name,
            package_name,
            buildscript_rule,
            env,
            features,
            version,
            local_manifest_dir,
        })
    }

    fn patch_from(&mut self, other: &BuildscriptRun) {
        // Patch env map
        for (k, v) in &other.env {
            self.env.entry(k.clone()).or_insert_with(|| v.clone());
        }
        // Patch features set
        let to_add: Vec<_> = other.features.difference(&self.features).cloned().collect();
        self.features.extend(to_add);
    }
}

pub fn parse_buck_file(file: &Utf8PathBuf) -> PyResult<Map<String, Rule>> {
    Python::attach(|py| {
        let buck = std::fs::read_to_string(file).expect("Failed to read BUCK file");
        let buck = buck.replace("cargo.rust", "cargo_rust");
        let python_code = format!(
            r#"
call_kwargs_list = []

def buckal_call(func):
    def wrapper(*args, **kwargs):
        global call_kwargs_list
        call_kwargs_list.append((func.__name__, kwargs))
        return func(*args, **kwargs)
    return wrapper

@buckal_call
def cargo_rust_library(*args, **kwargs):
    pass

@buckal_call
def cargo_rust_binary(*args, **kwargs):
    pass

@buckal_call
def buildscript_run(*args, **kwargs):
    pass

def glob(*args, **kwargs):
    return (glob.__name__, kwargs)

def load(*args, **kwargs):
    pass

        {}
"#,
            buck
        );

        let mut buck_rules: Map<String, Rule> = Map::new();

        let c_str = CString::new(python_code).unwrap();

        py.run(c_str.as_c_str(), None, None)?;

        let globals_binding = py.eval(c_str!("__import__('builtins').globals()"), None, None)?;
        let globals = globals_binding.downcast::<PyDict>()?;

        let kwargs_binding = globals
            .get_item("call_kwargs_list")
            .expect("call_kwargs_list not found")
            .unwrap();
        let kwargs_list = kwargs_binding.downcast::<PyList>()?;

        for tuple in kwargs_list.iter() {
            let tuple = tuple.downcast::<PyTuple>()?;
            let binding = tuple.get_item(0).unwrap();
            let func_name = binding.downcast::<PyString>()?;
            let func_name: &str = func_name.extract().unwrap();
            let binding = tuple.get_item(1).unwrap();
            let kwargs = binding.downcast::<PyDict>()?;

            match func_name {
                "cargo_rust_library" => {
                    let rule = CargoRustLibrary::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::CargoRustLibrary(rule));
                }
                "cargo_rust_binary" => {
                    let rule = CargoRustBinary::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::CargoRustBinary(rule));
                }
                "buildscript_run" => {
                    let rule = BuildscriptRun::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::BuildscriptRun(rule));
                }
                _ => panic!("Unknown function name: {}", func_name),
            }
        }

        Ok(buck_rules)
    })
}

pub fn patch_buck_rules(existing: &Map<String, Rule>, to_patch: &mut [Rule]) {
    for rule in to_patch.iter_mut() {
        match rule {
            Rule::CargoRustLibrary(new_rule) => {
                if let Some(Rule::CargoRustLibrary(existing_rule)) =
                    existing.get("cargo_rust_library")
                {
                    new_rule.patch_from(existing_rule);
                }
            }
            Rule::CargoRustBinary(new_rule) => {
                if let Some(Rule::CargoRustBinary(existing_rule)) =
                    existing.get("cargo_rust_binary")
                {
                    new_rule.patch_from(existing_rule);
                }
            }
            Rule::BuildscriptRun(new_rule) => {
                if let Some(Rule::BuildscriptRun(existing_rule)) = existing.get("buildscript_run") {
                    new_rule.patch_from(existing_rule);
                }
            }
            _ => {}
        }
    }
}
