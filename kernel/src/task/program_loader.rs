//! Phase 9 stored program manifest loader.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramKind {
    BuiltinAlias,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramManifest {
    pub name: String,
    pub kind: ProgramKind,
    pub entry: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProgram {
    pub name: String,
    pub source_path: String,
    pub kind: ProgramKind,
    pub entry: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramLoadError {
    InvalidVersion,
    MissingName,
    MissingEntry,
    UnsupportedKind,
    InvalidField,
    Storage,
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoaderStatus {
    pub program_count: usize,
    pub launch_count: u64,
    pub failed_launch_count: u64,
}

static LAUNCH_COUNT: AtomicU64 = AtomicU64::new(0);
static FAILED_LAUNCH_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn parse_manifest(contents: &str) -> Result<ProgramManifest, ProgramLoadError> {
    let mut lines = contents.lines();
    if lines.next() != Some("ares-exec-v1") {
        return Err(ProgramLoadError::InvalidVersion);
    }

    let mut name: Option<String> = None;
    let mut kind: Option<ProgramKind> = None;
    let mut entry: Option<String> = None;
    let mut description = String::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ProgramLoadError::InvalidField);
        };
        match key {
            "name" if !value.is_empty() => name = Some(value.to_string()),
            "kind" if value == "builtin-alias" => kind = Some(ProgramKind::BuiltinAlias),
            "kind" => return Err(ProgramLoadError::UnsupportedKind),
            "entry" if !value.is_empty() => entry = Some(value.to_string()),
            "description" => description = value.to_string(),
            _ => return Err(ProgramLoadError::InvalidField),
        }
    }

    Ok(ProgramManifest {
        name: name.ok_or(ProgramLoadError::MissingName)?,
        kind: kind.ok_or(ProgramLoadError::UnsupportedKind)?,
        entry: entry.ok_or(ProgramLoadError::MissingEntry)?,
        description,
    })
}

pub fn discover_programs() -> Vec<LoadedProgram> {
    let Ok(files) = crate::storage::list_files() else {
        return Vec::new();
    };

    let mut programs = Vec::new();
    for path in files {
        if !path.starts_with("/bin/") {
            continue;
        }
        let Ok(Some(contents)) = crate::storage::read_file(&path) else {
            continue;
        };
        let Ok(manifest) = parse_manifest(&contents) else {
            continue;
        };
        programs.push(LoadedProgram {
            name: manifest.name,
            source_path: path,
            kind: manifest.kind,
            entry: manifest.entry,
            description: manifest.description,
        });
    }
    programs
}

pub fn resolve_program(name: &str) -> Result<LoadedProgram, ProgramLoadError> {
    discover_programs()
        .into_iter()
        .find(|program| program.name == name)
        .ok_or(ProgramLoadError::NotFound)
}

pub fn program_info(name: &str) -> Result<LoadedProgram, ProgramLoadError> {
    resolve_program(name)
}

pub fn status() -> LoaderStatus {
    LoaderStatus {
        program_count: discover_programs().len(),
        launch_count: LAUNCH_COUNT.load(Ordering::Relaxed),
        failed_launch_count: FAILED_LAUNCH_COUNT.load(Ordering::Relaxed),
    }
}

pub fn record_launch_success() {
    LAUNCH_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_launch_failure() {
    FAILED_LAUNCH_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn manifest_for_builtin(name: &str, description: &str) -> String {
    format!(
        "ares-exec-v1\nname={}\nkind=builtin-alias\nentry={}\ndescription={}",
        name, name, description
    )
}

pub fn phase9_smoke_check() -> bool {
    let before = status().launch_count;
    let programs = discover_programs();
    let has_echo = programs.iter().any(|program| {
        program.name == "echo" && program.source_path == "/bin/echo" && program.entry == "echo"
    });
    let launch_ok = crate::task::userspace::run_program("echo", &["phase9-loader"])
        .map(|output| output == "phase9-loader")
        .unwrap_or(false);
    let after = status();
    has_echo && launch_ok && after.launch_count > before && after.program_count >= 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn valid_manifest_parses() {
        let manifest = parse_manifest(
            "ares-exec-v1\nname=echo\nkind=builtin-alias\nentry=echo\ndescription=Echo text",
        )
        .expect("manifest should parse");
        assert_eq!(manifest.name, "echo");
        assert_eq!(manifest.kind, ProgramKind::BuiltinAlias);
        assert_eq!(manifest.entry, "echo");
    }

    #[test_case]
    fn invalid_manifest_version_is_rejected() {
        assert_eq!(
            parse_manifest("bad-version\nname=echo\nkind=builtin-alias\nentry=echo"),
            Err(ProgramLoadError::InvalidVersion)
        );
    }

    #[test_case]
    fn missing_required_fields_are_rejected() {
        assert_eq!(
            parse_manifest("ares-exec-v1\nkind=builtin-alias\nentry=echo"),
            Err(ProgramLoadError::MissingName)
        );
        assert_eq!(
            parse_manifest("ares-exec-v1\nname=echo\nkind=builtin-alias"),
            Err(ProgramLoadError::MissingEntry)
        );
    }

    #[test_case]
    fn unsupported_kind_is_rejected() {
        assert_eq!(
            parse_manifest("ares-exec-v1\nname=x\nkind=elf\nentry=x"),
            Err(ProgramLoadError::UnsupportedKind)
        );
    }
}
