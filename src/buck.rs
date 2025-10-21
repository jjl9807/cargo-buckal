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
    HttpArchive(HttpArchive),
    FileGroup(FileGroup),
    CargoManifest(CargoManifest),
    RustLibrary(RustLibrary),
    RustBinary(RustBinary),
    BuildscriptRun(BuildscriptRun),
}

impl Rule {
    pub fn as_rust_rule_mut(&mut self) -> Option<&mut dyn RustRule> {
        match self {
            Rule::RustLibrary(inner) => Some(inner),
            Rule::RustBinary(inner) => Some(inner),
            _ => None,
        }
    }
}

pub trait RustRule {
    fn deps_mut(&mut self) -> &mut Set<String>;
    fn rustc_flags_mut(&mut self) -> &mut Set<String>;
    fn env_mut(&mut self) -> &mut Map<String, String>;
    fn named_deps_mut(&mut self) -> &mut Map<String, String>;
}

#[derive(Debug)]
pub struct Load {
    pub bzl: String,
    pub items: Set<String>,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename = "http_archive")]
pub struct HttpArchive {
    pub name: String,
    pub urls: Set<String>,
    pub sha256: String,
    #[serde(rename = "type")]
    pub _type: String,
    pub strip_prefix: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename = "cargo_manifest")]
pub struct CargoManifest {
    pub name: String,
    pub vendor: String,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename = "rust_library")]
pub struct RustLibrary {
    pub name: String,
    pub srcs: Set<String>,
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
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub named_deps: Map<String, String>,
    pub visibility: Set<String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub deps: Set<String>,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename = "rust_binary")]
pub struct RustBinary {
    pub name: String,
    pub srcs: Set<String>,
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
    #[serde(skip_serializing_if = "Map::is_empty")]
    pub named_deps: Map<String, String>,
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
    pub env_srcs: Set<String>,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub features: Set<String>,
    pub version: String,
    pub manifest_dir: String,
    #[serde(skip_serializing_if = "Set::is_empty")]
    pub visibility: Set<String>,
}

#[derive(Default, Debug)]
pub struct Glob {
    pub include: Set<String>,
    pub exclude: Set<String>,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename = "filegroup")]
pub struct FileGroup {
    pub name: String,
    pub srcs: Glob,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
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
    fn from_py_tuple(tuple: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let func_binding = tuple.get_item(0).unwrap();
        let func = func_binding.downcast::<PyString>().unwrap();
        assert_eq!(func.to_str().unwrap(), "glob");
        let args_binding = tuple.get_item(1).unwrap();
        let args = args_binding.downcast::<PyTuple>().unwrap();
        if args.len() > 1 {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "glob only supports one positional argument",
            ))
        } else if args.len() == 1 {
            let include_vec: Vec<String> = args
                .get_item(0)
                .expect("Expected one positional argument")
                .extract()
                .ok()
                .unwrap_or_default();
            let include: Set<String> = include_vec.into_iter().collect();
            Ok(Glob {
                include,
                exclude: Set::new(),
            })
        } else {
            let kwargs_binding = tuple.get_item(2).unwrap();
            let kwargs = kwargs_binding.downcast::<PyDict>().unwrap();
            let include_vec: Vec<String> = get_arg(kwargs, "include");
            let include: Set<String> = include_vec.into_iter().collect();
            let exclude_vec: Vec<String> = get_arg(kwargs, "exclude");
            let exclude: Set<String> = exclude_vec.into_iter().collect();
            Ok(Glob { include, exclude })
        }
    }
}

impl RustRule for RustLibrary {
    fn deps_mut(&mut self) -> &mut Set<String> {
        &mut self.deps
    }

    fn rustc_flags_mut(&mut self) -> &mut Set<String> {
        &mut self.rustc_flags
    }

    fn env_mut(&mut self) -> &mut Map<String, String> {
        &mut self.env
    }

    fn named_deps_mut(&mut self) -> &mut Map<String, String> {
        &mut self.named_deps
    }
}

impl RustRule for RustBinary {
    fn deps_mut(&mut self) -> &mut Set<String> {
        &mut self.deps
    }

    fn rustc_flags_mut(&mut self) -> &mut Set<String> {
        &mut self.rustc_flags
    }

    fn env_mut(&mut self) -> &mut Map<String, String> {
        &mut self.env
    }

    fn named_deps_mut(&mut self) -> &mut Map<String, String> {
        &mut self.named_deps
    }
}

impl RustLibrary {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = get_arg(kwargs, "name");
        let srcs_vec: Vec<String> = get_arg(kwargs, "srcs");
        let srcs: Set<String> = srcs_vec.into_iter().collect();
        let crate_name: String = get_arg(kwargs, "crate");
        let crate_root: String = get_arg(kwargs, "crate_root");
        let edition: String = get_arg(kwargs, "edition");
        let env: Map<String, String> = get_arg(kwargs, "env");
        let features_vec: Vec<String> = get_arg(kwargs, "features");
        let features: Set<String> = features_vec.into_iter().collect();
        let rustc_flags_vec: Vec<String> = get_arg(kwargs, "rustc_flags");
        let rustc_flags: Set<String> = rustc_flags_vec.into_iter().collect();
        let proc_macro: Option<bool> = get_arg(kwargs, "proc_macro");
        let named_deps: Map<String, String> = get_arg(kwargs, "named_deps");
        let visibility_vec: Vec<String> = get_arg(kwargs, "visibility");
        let visibility: Set<String> = visibility_vec.into_iter().collect();
        let deps_vec: Vec<String> = get_arg(kwargs, "deps");
        let deps: Set<String> = deps_vec.into_iter().collect();
        Ok(RustLibrary {
            name,
            srcs,
            crate_name,
            crate_root,
            edition,
            env,
            features,
            rustc_flags,
            proc_macro,
            named_deps,
            visibility,
            deps,
        })
    }

    fn patch_from(&mut self, other: &RustLibrary) {
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
    }
}

impl RustBinary {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = get_arg(kwargs, "name");
        let srcs_vec: Vec<String> = get_arg(kwargs, "srcs");
        let srcs: Set<String> = srcs_vec.into_iter().collect();
        let crate_name: String = get_arg(kwargs, "crate");
        let crate_root: String = get_arg(kwargs, "crate_root");
        let edition: String = get_arg(kwargs, "edition");
        let env: Map<String, String> = get_arg(kwargs, "env");
        let features_vec: Vec<String> = get_arg(kwargs, "features");
        let features: Set<String> = features_vec.into_iter().collect();
        let rustc_flags_vec: Vec<String> = get_arg(kwargs, "rustc_flags");
        let rustc_flags: Set<String> = rustc_flags_vec.into_iter().collect();
        let named_deps: Map<String, String> = get_arg(kwargs, "named_deps");
        let visibility_vec: Vec<String> = get_arg(kwargs, "visibility");
        let visibility: Set<String> = visibility_vec.into_iter().collect();
        let deps_vec: Vec<String> = get_arg(kwargs, "deps");
        let deps: Set<String> = deps_vec.into_iter().collect();
        Ok(RustBinary {
            name,
            srcs,
            crate_name,
            crate_root,
            edition,
            env,
            features,
            rustc_flags,
            named_deps,
            visibility,
            deps,
        })
    }

    fn patch_from(&mut self, other: &RustBinary) {
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
    }
}

impl BuildscriptRun {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = get_arg(kwargs, "name");
        let package_name: String = get_arg(kwargs, "package_name");
        let buildscript_rule: String = get_arg(kwargs, "buildscript_rule");
        let env: Map<String, String> = get_arg(kwargs, "env");
        let env_srcs_vec: Vec<String> = get_arg(kwargs, "env_srcs");
        let env_srcs: Set<String> = env_srcs_vec.into_iter().collect();
        let features_vec: Vec<String> = get_arg(kwargs, "features");
        let features: Set<String> = features_vec.into_iter().collect();
        let version: String = get_arg(kwargs, "version");
        let manifest_dir: String = get_arg(kwargs, "manifest_dir");
        let visibility_vec: Vec<String> = get_arg(kwargs, "visibility");
        let visibility: Set<String> = visibility_vec.into_iter().collect();
        Ok(BuildscriptRun {
            name,
            package_name,
            buildscript_rule,
            env,
            env_srcs,
            features,
            version,
            manifest_dir,
            visibility,
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
        // Patch visibility set
        let to_add: Vec<_> = other
            .visibility
            .difference(&self.visibility)
            .cloned()
            .collect();
        self.visibility.extend(to_add);
    }
}

impl HttpArchive {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = get_arg(kwargs, "name");
        let urls_vec: Vec<String> = get_arg(kwargs, "urls");
        let urls: Set<String> = urls_vec.into_iter().collect();
        let sha256: String = get_arg(kwargs, "sha256");
        let _type: String = get_arg(kwargs, "type");
        let strip_prefix: String = get_arg(kwargs, "strip_prefix");
        let out: Option<String> = get_arg(kwargs, "out");
        Ok(HttpArchive {
            name,
            urls,
            sha256,
            _type,
            strip_prefix,
            out,
        })
    }
}

impl FileGroup {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = get_arg(kwargs, "name");
        let srcs_tuple_binding = kwargs
            .get_item("srcs")
            .expect("Expected 'srcs' argument")
            .unwrap();
        let srcs_tuple = srcs_tuple_binding.downcast::<PyTuple>().unwrap();
        let srcs = Glob::from_py_tuple(srcs_tuple)?;
        let out: Option<String> = get_arg(kwargs, "out");
        Ok(FileGroup { name, srcs, out })
    }
}

impl CargoManifest {
    fn from_py_dict(kwargs: &Bound<'_, PyDict>) -> PyResult<Self> {
        let name: String = get_arg(kwargs, "name");
        let vendor: String = get_arg(kwargs, "vendor");
        Ok(CargoManifest { name, vendor })
    }
}

pub fn parse_buck_file(file: &Utf8PathBuf) -> PyResult<Map<String, Rule>> {
    Python::attach(|py| {
        let buck = std::fs::read_to_string(file).expect("Failed to read BUCK file");
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
def rust_library(*args, **kwargs):
    pass

@buckal_call
def rust_binary(*args, **kwargs):
    pass

@buckal_call
def buildscript_run(*args, **kwargs):
    pass

@buckal_call
def http_archive(*args, **kwargs):
    pass

@buckal_call
def filegroup(*args, **kwargs):
    pass

@buckal_call
def cargo_manifest(*args, **kwargs):
    pass

def glob(*args, **kwargs):
    return (glob.__name__, args, kwargs)

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
                "rust_library" => {
                    let rule = RustLibrary::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::RustLibrary(rule));
                }
                "rust_binary" => {
                    let rule = RustBinary::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::RustBinary(rule));
                }
                "buildscript_run" => {
                    let rule = BuildscriptRun::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::BuildscriptRun(rule));
                }
                "http_archive" => {
                    let rule = HttpArchive::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::HttpArchive(rule));
                }
                "filegroup" => {
                    let rule = FileGroup::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::FileGroup(rule));
                }
                "cargo_manifest" => {
                    let rule = CargoManifest::from_py_dict(kwargs)?;
                    buck_rules.insert(func_name.to_string(), Rule::CargoManifest(rule));
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
            Rule::RustLibrary(new_rule) => {
                if let Some(Rule::RustLibrary(existing_rule)) = existing.get("rust_library") {
                    new_rule.patch_from(existing_rule);
                }
            }
            Rule::RustBinary(new_rule) => {
                if let Some(Rule::RustBinary(existing_rule)) = existing.get("rust_binary") {
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

fn get_arg<'a, T>(kwargs: &Bound<'a, PyDict>, key: &str) -> T
where
    T: Default + FromPyObject<'a>,
{
    kwargs
        .get_item(key)
        .unwrap_or_else(|_| panic!("Expected '{}' argument", key))
        .and_then(|v| v.extract().ok())
        .unwrap_or_default()
}
