use std::collections::BTreeSet;

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Platform: u32 {
        const WINDOWS = 0b0001;
        const MACOS   = 0b0010;
        const LINUX   = 0b0100;
    }
}

impl Platform {
    pub fn to_buck(self) -> BTreeSet<String> {
        let mut platforms = BTreeSet::new();
        if self.contains(Platform::WINDOWS) {
            platforms.insert("prelude//os:windows".to_string());
        }
        if self.contains(Platform::MACOS) {
            platforms.insert("prelude//os:macos".to_string());
        }
        if self.contains(Platform::LINUX) {
            platforms.insert("prelude//os:linux".to_string());
        }
        platforms
    }
}

static PACKAGE_PLATFORMS: phf::Map<&'static str, Platform> = phf::phf_map! {
    "hyper-named-pipe" => Platform::WINDOWS,
    "system-configuration" => Platform::MACOS,
    "windows-future" => Platform::WINDOWS,
    "windows" => Platform::WINDOWS,
    "winreg" => Platform::WINDOWS,
};

pub fn lookup_platforms(package_name: &str) -> Option<Platform> {
    PACKAGE_PLATFORMS.get(package_name).copied()
}
