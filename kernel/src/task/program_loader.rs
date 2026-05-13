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
    Elf64Image,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramTrust {
    System,
    User,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramManifest {
    pub name: String,
    pub kind: ProgramKind,
    pub entry: String,
    pub image_path: Option<String>,
    pub description: String,
    pub requires_execute: bool,
    pub trust: ProgramTrust,
    pub owner: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProgram {
    pub name: String,
    pub source_path: String,
    pub kind: ProgramKind,
    pub entry: String,
    pub image_path: Option<String>,
    pub description: String,
    pub requires_execute: bool,
    pub trust: ProgramTrust,
    pub owner: String,
    pub image: Option<crate::exec_image::ExecutableImage>,
    pub image_error: Option<crate::exec_image::ImageLoadError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramLoadError {
    InvalidVersion,
    MissingName,
    MissingEntry,
    UnsupportedKind,
    UnsupportedTrust,
    UnsupportedRequirement,
    MissingImage,
    InvalidField,
    Storage,
    NotFound,
    PermissionDenied,
    UnsupportedExecution,
    ImageInvalid,
    LoadPlanRejected,
    MappingRejected,
    FrameBackingRejected,
    PageTableRejected,
    UserContextRejected,
    Ring3TrampolineRejected,
    UserSyscallRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoaderStatus {
    pub program_count: usize,
    pub launch_count: u64,
    pub failed_launch_count: u64,
    pub denied_launch_count: u64,
    pub image_count: usize,
    pub valid_image_count: usize,
    pub invalid_image_count: usize,
    pub unsupported_execution_count: u64,
    pub prepared_image_count: u64,
    pub rejected_load_plan_count: u64,
    pub total_planned_pages: u64,
    pub mapped_image_count: u64,
    pub rejected_mapping_count: u64,
    pub total_mapped_pages: u64,
    pub copied_bytes: u64,
    pub zero_filled_bytes: u64,
    pub frame_backed_image_count: u64,
    pub rejected_frame_backing_count: u64,
    pub total_frame_backed_pages: u64,
    pub user_page_table_count: u64,
    pub rejected_user_page_table_count: u64,
    pub total_user_page_table_pages: u64,
    pub user_context_count: u64,
    pub rejected_user_context_count: u64,
    pub ring3_entry_count: u64,
    pub ring3_trap_count: u64,
    pub rejected_ring3_count: u64,
    pub user_syscall_count: u64,
    pub user_syscall_return_count: u64,
    pub rejected_user_syscall_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedProgramImage {
    pub program: LoadedProgram,
    pub image: crate::exec_image::ExecutableImage,
    pub load_plan: crate::load_plan::LoadPlan,
    pub address_space: crate::address_space::AddressSpaceDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedProgramImage {
    pub prepared: PreparedProgramImage,
    pub mapped: crate::mapping_stub::MappedImage,
    pub address_space: crate::address_space::AddressSpaceDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameBackedProgramImage {
    pub mapped: MappedProgramImage,
    pub backed: crate::frame_backing::FrameBackedImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPageTableProgramImage {
    pub backed: FrameBackedProgramImage,
    pub page_table: crate::user_memory::InactiveUserPageTable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserContextProgramImage {
    pub page_table: UserPageTableProgramImage,
    pub context: crate::user_context::UserContextDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ring3TrampolineProgramImage {
    pub user_context: UserContextProgramImage,
    pub result: crate::ring3_trampoline::Ring3TrampolineResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSyscallProgramImage {
    pub ring3: Ring3TrampolineProgramImage,
    pub syscall_return: crate::user_syscall::UserSyscallReturn,
}

static LAUNCH_COUNT: AtomicU64 = AtomicU64::new(0);
static FAILED_LAUNCH_COUNT: AtomicU64 = AtomicU64::new(0);
static DENIED_LAUNCH_COUNT: AtomicU64 = AtomicU64::new(0);
static UNSUPPORTED_EXECUTION_COUNT: AtomicU64 = AtomicU64::new(0);
static PREPARED_IMAGE_COUNT: AtomicU64 = AtomicU64::new(0);
static REJECTED_LOAD_PLAN_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_PLANNED_PAGES: AtomicU64 = AtomicU64::new(0);
static MAPPED_IMAGE_COUNT: AtomicU64 = AtomicU64::new(0);
static REJECTED_MAPPING_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_MAPPED_PAGES: AtomicU64 = AtomicU64::new(0);
static COPIED_BYTES: AtomicU64 = AtomicU64::new(0);
static ZERO_FILLED_BYTES: AtomicU64 = AtomicU64::new(0);
static FRAME_BACKED_IMAGE_COUNT: AtomicU64 = AtomicU64::new(0);
static REJECTED_FRAME_BACKING_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_FRAME_BACKED_PAGES: AtomicU64 = AtomicU64::new(0);
static USER_PAGE_TABLE_COUNT: AtomicU64 = AtomicU64::new(0);
static REJECTED_USER_PAGE_TABLE_COUNT: AtomicU64 = AtomicU64::new(0);
static TOTAL_USER_PAGE_TABLE_PAGES: AtomicU64 = AtomicU64::new(0);
static USER_CONTEXT_COUNT: AtomicU64 = AtomicU64::new(0);
static REJECTED_USER_CONTEXT_COUNT: AtomicU64 = AtomicU64::new(0);
static RING3_ENTRY_COUNT: AtomicU64 = AtomicU64::new(0);
static RING3_TRAP_COUNT: AtomicU64 = AtomicU64::new(0);
static REJECTED_RING3_COUNT: AtomicU64 = AtomicU64::new(0);
static USER_SYSCALL_COUNT: AtomicU64 = AtomicU64::new(0);
static USER_SYSCALL_RETURN_COUNT: AtomicU64 = AtomicU64::new(0);
static REJECTED_USER_SYSCALL_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn parse_manifest(contents: &str) -> Result<ProgramManifest, ProgramLoadError> {
    let mut lines = contents.lines();
    if lines.next() != Some("ares-exec-v1") {
        return Err(ProgramLoadError::InvalidVersion);
    }

    let mut name: Option<String> = None;
    let mut kind: Option<ProgramKind> = None;
    let mut entry: Option<String> = None;
    let mut image_path: Option<String> = None;
    let mut description = String::new();
    let mut requires_execute = true;
    let mut trust = ProgramTrust::User;
    let mut owner = String::from("user");

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
            "kind" if value == "elf64-image" => kind = Some(ProgramKind::Elf64Image),
            "kind" => return Err(ProgramLoadError::UnsupportedKind),
            "entry" if !value.is_empty() => entry = Some(value.to_string()),
            "image" if !value.is_empty() => image_path = Some(value.to_string()),
            "description" => description = value.to_string(),
            "requires" if value == "execute" => requires_execute = true,
            "requires" => return Err(ProgramLoadError::UnsupportedRequirement),
            "trust" if value == "system" => trust = ProgramTrust::System,
            "trust" if value == "user" => trust = ProgramTrust::User,
            "trust" => return Err(ProgramLoadError::UnsupportedTrust),
            "owner" => owner = value.to_string(),
            _ => return Err(ProgramLoadError::InvalidField),
        }
    }

    let kind = kind.ok_or(ProgramLoadError::UnsupportedKind)?;
    if kind == ProgramKind::Elf64Image && image_path.is_none() {
        return Err(ProgramLoadError::MissingImage);
    }

    Ok(ProgramManifest {
        name: name.ok_or(ProgramLoadError::MissingName)?,
        kind,
        entry: entry.ok_or(ProgramLoadError::MissingEntry)?,
        image_path,
        description,
        requires_execute,
        trust,
        owner,
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
        let (image, image_error) = validate_manifest_image(&manifest);
        programs.push(LoadedProgram {
            name: manifest.name,
            source_path: path,
            kind: manifest.kind,
            entry: manifest.entry,
            image_path: manifest.image_path,
            description: manifest.description,
            requires_execute: manifest.requires_execute,
            trust: manifest.trust,
            owner: manifest.owner,
            image,
            image_error,
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

pub fn resolve_program_for(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<LoadedProgram, ProgramLoadError> {
    let program = resolve_program(name)?;
    if program.requires_execute {
        crate::storage::can_execute(credentials, &program.source_path).map_err(|_| {
            record_launch_denied();
            ProgramLoadError::PermissionDenied
        })?;
    }
    if let Some(image_path) = &program.image_path {
        crate::storage::can_execute(credentials, image_path).map_err(|_| {
            record_launch_denied();
            ProgramLoadError::PermissionDenied
        })?;
    }
    if program.kind == ProgramKind::Elf64Image {
        if program.image_error.is_some() {
            record_launch_failure();
            return Err(ProgramLoadError::ImageInvalid);
        }
        record_unsupported_execution();
        return Err(ProgramLoadError::UnsupportedExecution);
    }
    Ok(program)
}

pub fn program_info(name: &str) -> Result<LoadedProgram, ProgramLoadError> {
    resolve_program(name)
}

pub fn status() -> LoaderStatus {
    let programs = discover_programs();
    let image_count = programs
        .iter()
        .filter(|program| program.kind == ProgramKind::Elf64Image)
        .count();
    let valid_image_count = programs
        .iter()
        .filter(|program| program.kind == ProgramKind::Elf64Image && program.image.is_some())
        .count();
    LoaderStatus {
        program_count: programs.len(),
        launch_count: LAUNCH_COUNT.load(Ordering::Relaxed),
        failed_launch_count: FAILED_LAUNCH_COUNT.load(Ordering::Relaxed),
        denied_launch_count: DENIED_LAUNCH_COUNT.load(Ordering::Relaxed),
        image_count,
        valid_image_count,
        invalid_image_count: image_count.saturating_sub(valid_image_count),
        unsupported_execution_count: UNSUPPORTED_EXECUTION_COUNT.load(Ordering::Relaxed),
        prepared_image_count: PREPARED_IMAGE_COUNT.load(Ordering::Relaxed),
        rejected_load_plan_count: REJECTED_LOAD_PLAN_COUNT.load(Ordering::Relaxed),
        total_planned_pages: TOTAL_PLANNED_PAGES.load(Ordering::Relaxed),
        mapped_image_count: MAPPED_IMAGE_COUNT.load(Ordering::Relaxed),
        rejected_mapping_count: REJECTED_MAPPING_COUNT.load(Ordering::Relaxed),
        total_mapped_pages: TOTAL_MAPPED_PAGES.load(Ordering::Relaxed),
        copied_bytes: COPIED_BYTES.load(Ordering::Relaxed),
        zero_filled_bytes: ZERO_FILLED_BYTES.load(Ordering::Relaxed),
        frame_backed_image_count: FRAME_BACKED_IMAGE_COUNT.load(Ordering::Relaxed),
        rejected_frame_backing_count: REJECTED_FRAME_BACKING_COUNT.load(Ordering::Relaxed),
        total_frame_backed_pages: TOTAL_FRAME_BACKED_PAGES.load(Ordering::Relaxed),
        user_page_table_count: USER_PAGE_TABLE_COUNT.load(Ordering::Relaxed),
        rejected_user_page_table_count: REJECTED_USER_PAGE_TABLE_COUNT.load(Ordering::Relaxed),
        total_user_page_table_pages: TOTAL_USER_PAGE_TABLE_PAGES.load(Ordering::Relaxed),
        user_context_count: USER_CONTEXT_COUNT.load(Ordering::Relaxed),
        rejected_user_context_count: REJECTED_USER_CONTEXT_COUNT.load(Ordering::Relaxed),
        ring3_entry_count: RING3_ENTRY_COUNT.load(Ordering::Relaxed),
        ring3_trap_count: RING3_TRAP_COUNT.load(Ordering::Relaxed),
        rejected_ring3_count: REJECTED_RING3_COUNT.load(Ordering::Relaxed),
        user_syscall_count: USER_SYSCALL_COUNT.load(Ordering::Relaxed),
        user_syscall_return_count: USER_SYSCALL_RETURN_COUNT.load(Ordering::Relaxed),
        rejected_user_syscall_count: REJECTED_USER_SYSCALL_COUNT.load(Ordering::Relaxed),
    }
}

pub fn record_launch_success() {
    LAUNCH_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_launch_failure() {
    FAILED_LAUNCH_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_launch_denied() {
    DENIED_LAUNCH_COUNT.fetch_add(1, Ordering::Relaxed);
    FAILED_LAUNCH_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn record_unsupported_execution() {
    UNSUPPORTED_EXECUTION_COUNT.fetch_add(1, Ordering::Relaxed);
    FAILED_LAUNCH_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn manifest_for_builtin(name: &str, description: &str) -> String {
    format!(
        "ares-exec-v1\nname={}\nkind=builtin-alias\nentry={}\nrequires=execute\ntrust=system\nowner=admin\ndescription={}",
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

pub fn validate_program_image(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<crate::exec_image::ExecutableImage, ProgramLoadError> {
    let program = resolve_program(name)?;
    if program.kind != ProgramKind::Elf64Image {
        return Ok(crate::exec_image::builtin_image(
            &program.name,
            &program.source_path,
            program.trust,
            owner_id_for_manifest(&program.owner),
        ));
    }
    crate::storage::can_execute(credentials, &program.source_path)
        .map_err(|_| ProgramLoadError::PermissionDenied)?;
    let image_path = program.image_path.as_ref().ok_or(ProgramLoadError::MissingImage)?;
    crate::storage::can_execute(credentials, image_path)
        .map_err(|_| ProgramLoadError::PermissionDenied)?;
    program.image.ok_or(ProgramLoadError::ImageInvalid)
}

pub fn phase11_smoke_check() -> bool {
    let initial_status = status();
    let before = initial_status.unsupported_execution_count;
    let validate_ok = validate_program_image(crate::security::Credentials::shell_user(), "hello")
        .map(|image| {
            crate::address_space::descriptor_for_image(
                crate::address_space::AddressSpaceId::from_raw(1),
                &image,
            )
            .map(|descriptor| !descriptor.regions.is_empty())
            .unwrap_or(false)
        })
        .unwrap_or(false);
    let blocked_ok = crate::task::userspace::run_program("hello", &[])
        .map(|_| false)
        .unwrap_or(true)
        && status().unsupported_execution_count > before;
    validate_ok && initial_status.image_count >= 1 && initial_status.valid_image_count >= 1 && blocked_ok
}

pub fn prepare_program_image(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<PreparedProgramImage, ProgramLoadError> {
    let program = resolve_program(name)?;
    if program.kind != ProgramKind::Elf64Image {
        return Err(ProgramLoadError::UnsupportedKind);
    }
    crate::storage::can_execute(credentials, &program.source_path)
        .map_err(|_| ProgramLoadError::PermissionDenied)?;
    let image_path = program.image_path.as_ref().ok_or(ProgramLoadError::MissingImage)?;
    crate::storage::can_execute(credentials, image_path)
        .map_err(|_| ProgramLoadError::PermissionDenied)?;
    let image = program.image.clone().ok_or(ProgramLoadError::ImageInvalid)?;
    let load_plan = crate::load_plan::build_load_plan(&image).map_err(|_| {
        REJECTED_LOAD_PLAN_COUNT.fetch_add(1, Ordering::Relaxed);
        ProgramLoadError::LoadPlanRejected
    })?;
    let address_space_id = crate::address_space::AddressSpaceId::from_raw(
        PREPARED_IMAGE_COUNT.load(Ordering::Relaxed).saturating_add(1),
    );
    let address_space = crate::address_space::descriptor_for_load_plan(address_space_id, &load_plan)
        .map_err(|_| {
            REJECTED_LOAD_PLAN_COUNT.fetch_add(1, Ordering::Relaxed);
            ProgramLoadError::LoadPlanRejected
        })?;

    PREPARED_IMAGE_COUNT.fetch_add(1, Ordering::Relaxed);
    TOTAL_PLANNED_PAGES.fetch_add(load_plan.total_pages as u64, Ordering::Relaxed);
    record_prepared_process(credentials, &program, &image, &load_plan, address_space_id);

    Ok(PreparedProgramImage {
        program,
        image,
        load_plan,
        address_space,
    })
}

pub fn phase12_smoke_check() -> bool {
    let before = status();
    let prepared = prepare_program_image(crate::security::Credentials::shell_user(), "hello")
        .map(|prepared| prepared.load_plan.total_pages > 0 && !prepared.address_space.regions.is_empty())
        .unwrap_or(false);
    let blocked = crate::task::userspace::run_program("hello", &[])
        .map(|_| false)
        .unwrap_or(true);
    let after = status();
    prepared
        && blocked
        && after.prepared_image_count > before.prepared_image_count
        && after.total_planned_pages > before.total_planned_pages
        && after.unsupported_execution_count > before.unsupported_execution_count
}

pub fn map_prepared_program(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<MappedProgramImage, ProgramLoadError> {
    let prepared = prepare_program_image(credentials, name)?;
    let mapping = crate::mapping_stub::register_mapping(
        credentials,
        prepared.address_space.id,
        &prepared.load_plan,
    )
    .map_err(|_| {
        REJECTED_MAPPING_COUNT.fetch_add(1, Ordering::Relaxed);
        ProgramLoadError::MappingRejected
    })?;
    let mapped_address_space = crate::address_space::descriptor_for_mapped_image(&mapping);

    MAPPED_IMAGE_COUNT.fetch_add(1, Ordering::Relaxed);
    TOTAL_MAPPED_PAGES.fetch_add(mapping.total_pages as u64, Ordering::Relaxed);
    COPIED_BYTES.fetch_add(mapping.copied_bytes as u64, Ordering::Relaxed);
    ZERO_FILLED_BYTES.fetch_add(mapping.zero_filled_bytes as u64, Ordering::Relaxed);
    record_mapped_process(credentials, &prepared, &mapping);

    Ok(MappedProgramImage {
        prepared,
        mapped: mapping,
        address_space: mapped_address_space,
    })
}

pub fn phase13_smoke_check() -> bool {
    let before = status();
    let mapped = map_prepared_program(crate::security::Credentials::shell_user(), "hello")
        .map(|mapped| {
            mapped.mapped.total_pages > 0
                && mapped.mapped.copied_bytes > 0
                && mapped.mapped.zero_filled_bytes > 0
                && mapped.address_space.reservation.mapping_state
                    == crate::address_space::MappingState::MappedStub
        })
        .unwrap_or(false);
    let blocked = crate::task::userspace::run_program("hello", &[])
        .map(|_| false)
        .unwrap_or(true);
    let after = status();
    mapped
        && blocked
        && after.mapped_image_count > before.mapped_image_count
        && after.total_mapped_pages > before.total_mapped_pages
        && after.copied_bytes > before.copied_bytes
        && after.zero_filled_bytes > before.zero_filled_bytes
}

pub fn back_mapped_program(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<FrameBackedProgramImage, ProgramLoadError> {
    let mapped = map_prepared_program(credentials, name)?;
    let backed = crate::frame_backing::back_mapped_image(&mapped.mapped).map_err(|_| {
        REJECTED_FRAME_BACKING_COUNT.fetch_add(1, Ordering::Relaxed);
        ProgramLoadError::FrameBackingRejected
    })?;

    FRAME_BACKED_IMAGE_COUNT.fetch_add(1, Ordering::Relaxed);
    TOTAL_FRAME_BACKED_PAGES.fetch_add(backed.total_pages as u64, Ordering::Relaxed);
    record_frame_backed_process(credentials, &mapped.prepared, &backed);

    Ok(FrameBackedProgramImage { mapped, backed })
}

pub fn phase15_smoke_check() -> bool {
    let before = status();
    let before_frames = crate::frame_ownership::status();
    let backed = back_mapped_program(crate::security::Credentials::shell_user(), "hello")
        .map(|backed| {
            backed.backed.total_pages > 0
                && backed.backed.copied_bytes > 0
                && backed.backed.zero_filled_bytes > 0
                && backed.backed.state == crate::address_space::MappingState::FrameBacked
        })
        .unwrap_or(false);
    let after = status();
    let after_frames = crate::frame_ownership::status();
    backed
        && after.frame_backed_image_count > before.frame_backed_image_count
        && after.total_frame_backed_pages > before.total_frame_backed_pages
        && after_frames.allocated_frames > before_frames.allocated_frames
}

pub fn build_user_page_table(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<UserPageTableProgramImage, ProgramLoadError> {
    let backed = back_mapped_program(credentials, name)?;
    let id = crate::user_memory::UserPageTableId::from_raw(
        USER_PAGE_TABLE_COUNT.load(Ordering::Relaxed).saturating_add(1),
    );
    let page_table =
        crate::user_memory::build_inactive_page_table(id, &backed.backed).map_err(|_| {
            REJECTED_USER_PAGE_TABLE_COUNT.fetch_add(1, Ordering::Relaxed);
            ProgramLoadError::PageTableRejected
        })?;

    USER_PAGE_TABLE_COUNT.fetch_add(1, Ordering::Relaxed);
    TOTAL_USER_PAGE_TABLE_PAGES.fetch_add(page_table.mapped_pages as u64, Ordering::Relaxed);
    record_page_table_process(credentials, &backed.mapped.prepared, &backed.backed, &page_table);

    Ok(UserPageTableProgramImage { backed, page_table })
}

pub fn phase16_smoke_check() -> bool {
    let before = status();
    let built = build_user_page_table(crate::security::Credentials::shell_user(), "hello")
        .map(|built| {
            built.page_table.mapped_pages > 0
                && crate::user_memory::translate(
                    &built.page_table,
                    built.backed.backed.regions[0].pages[0].virtual_address,
                )
                .is_some()
                && !built.page_table.cr3_switch_ready
        })
        .unwrap_or(false);
    let after = status();
    built
        && after.user_page_table_count > before.user_page_table_count
        && after.total_user_page_table_pages > before.total_user_page_table_pages
}

pub fn prepare_user_context(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<UserContextProgramImage, ProgramLoadError> {
    let page_table = build_user_page_table(credentials, name)?;
    let context = crate::user_context::build_user_context(
        &page_table.page_table,
        page_table.backed.mapped.prepared.load_plan.entry_point,
        crate::gdt::user_selectors(),
    )
    .map_err(|_| {
        REJECTED_USER_CONTEXT_COUNT.fetch_add(1, Ordering::Relaxed);
        ProgramLoadError::UserContextRejected
    })?;

    USER_CONTEXT_COUNT.fetch_add(1, Ordering::Relaxed);
    record_user_context_process(
        credentials,
        &page_table.backed.mapped.prepared,
        &page_table.backed.backed,
        &context,
    );

    Ok(UserContextProgramImage {
        page_table,
        context,
    })
}

pub fn phase17_smoke_check() -> bool {
    let before = status();
    let prepared = prepare_user_context(crate::security::Credentials::shell_user(), "hello")
        .map(|prepared| {
            prepared.context.selectors_ready
                && prepared.context.entry_ready
                && !prepared.context.ring3_entered
                && prepared.context.entry.rip == prepared.page_table.backed.mapped.prepared.load_plan.entry_point
        })
        .unwrap_or(false);
    let after = status();
    prepared && after.user_context_count > before.user_context_count
}

pub fn enter_controlled_ring3_trampoline(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<Ring3TrampolineProgramImage, ProgramLoadError> {
    let user_context = prepare_user_context(credentials, name)?;
    let result = crate::ring3_trampoline::enter_controlled_trampoline(&user_context.context)
        .map_err(|_| {
            REJECTED_RING3_COUNT.fetch_add(1, Ordering::Relaxed);
            ProgramLoadError::Ring3TrampolineRejected
        })?;

    RING3_ENTRY_COUNT.fetch_add(1, Ordering::Relaxed);
    if result.trapped_back {
        RING3_TRAP_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    record_ring3_trap_process(
        credentials,
        &user_context.page_table.backed.mapped.prepared,
        &user_context.page_table.backed.backed,
        &result,
    );

    Ok(Ring3TrampolineProgramImage {
        user_context,
        result,
    })
}

pub fn phase18_smoke_check() -> bool {
    let before = status();
    let entered = enter_controlled_ring3_trampoline(
        crate::security::Credentials::shell_user(),
        "hello",
    )
    .map(|entered| entered.result.ring3_entered && entered.result.trapped_back)
    .unwrap_or(false);
    let after = status();
    entered
        && after.ring3_entry_count > before.ring3_entry_count
        && after.ring3_trap_count > before.ring3_trap_count
}

pub fn run_user_syscall_probe(
    credentials: crate::security::Credentials,
    name: &str,
) -> Result<UserSyscallProgramImage, ProgramLoadError> {
    let ring3 = enter_controlled_ring3_trampoline(credentials, name)?;
    let syscall_return =
        crate::user_syscall::dispatch_from_user(crate::user_syscall::tick_probe_frame()).map_err(
            |_| {
                REJECTED_USER_SYSCALL_COUNT.fetch_add(1, Ordering::Relaxed);
                ProgramLoadError::UserSyscallRejected
            },
        )?;

    USER_SYSCALL_COUNT.fetch_add(1, Ordering::Relaxed);
    if syscall_return.returned_to_user {
        USER_SYSCALL_RETURN_COUNT.fetch_add(1, Ordering::Relaxed);
    }
    record_user_syscall_process(
        credentials,
        &ring3.user_context.page_table.backed.mapped.prepared,
        &ring3.user_context.page_table.backed.backed,
        &syscall_return,
    );

    Ok(UserSyscallProgramImage {
        ring3,
        syscall_return,
    })
}

pub fn phase19_smoke_check() -> bool {
    let before = status();
    let returned = run_user_syscall_probe(crate::security::Credentials::shell_user(), "hello")
        .map(|probe| probe.syscall_return.returned_to_user && probe.syscall_return.error.is_none())
        .unwrap_or(false);
    let after = status();
    returned
        && after.user_syscall_count > before.user_syscall_count
        && after.user_syscall_return_count > before.user_syscall_return_count
}

fn validate_manifest_image(
    manifest: &ProgramManifest,
) -> (Option<crate::exec_image::ExecutableImage>, Option<crate::exec_image::ImageLoadError>) {
    if manifest.kind != ProgramKind::Elf64Image {
        return (None, None);
    }
    let Some(image_path) = &manifest.image_path else {
        return (None, Some(crate::exec_image::ImageLoadError::InvalidHeader));
    };
    let Ok(Some(contents)) = crate::storage::read_file(image_path) else {
        return (None, Some(crate::exec_image::ImageLoadError::InvalidHeader));
    };
    match crate::exec_image::parse_elf64_image(
        &manifest.name,
        image_path,
        contents.as_bytes(),
        manifest.trust,
        owner_id_for_manifest(&manifest.owner),
    ) {
        Ok(image) => (Some(image), None),
        Err(err) => (None, Some(err)),
    }
}

fn owner_id_for_manifest(owner: &str) -> crate::security::UserId {
    match owner {
        "admin" => crate::security::Credentials::admin().user,
        "kernel" => crate::security::Credentials::kernel().user,
        "guest" => crate::security::Credentials::guest().user,
        _ => crate::security::Credentials::shell_user().user,
    }
}

fn record_prepared_process(
    credentials: crate::security::Credentials,
    program: &LoadedProgram,
    image: &crate::exec_image::ExecutableImage,
    load_plan: &crate::load_plan::LoadPlan,
    address_space_id: crate::address_space::AddressSpaceId,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let metadata = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&program.source_path),
        format: image.format,
        entry_point: image.entry_point,
        segment_count: image.segments.len(),
        address_space_id: Some(address_space_id),
        trust: image.trust,
        owner: credentials,
    };
    let load = crate::task::process::ProcessLoadMetadata {
        state: crate::task::process::ProcessLoadState::Prepared,
        source_path: static_source_path(&image.source_path),
        entry_point: load_plan.entry_point,
        planned_pages: load_plan.total_pages,
        region_count: load_plan.regions.len(),
        stack_pages: load_plan.stack_pages,
        mapping_id: None,
        copied_bytes: 0,
        zero_filled_bytes: 0,
        executable_pages: 0,
    };
    if let Some(pid) = crate::task::process::create_kernel_process_with_metadata(
        "image-prepare",
        tick,
        credentials,
        metadata,
        load,
    ) {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Blocked);
    }
}

fn record_mapped_process(
    credentials: crate::security::Credentials,
    prepared: &PreparedProgramImage,
    mapped: &crate::mapping_stub::MappedImage,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let metadata = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&prepared.program.source_path),
        format: prepared.image.format,
        entry_point: prepared.image.entry_point,
        segment_count: prepared.image.segments.len(),
        address_space_id: Some(mapped.address_space_id),
        trust: prepared.image.trust,
        owner: credentials,
    };
    let load = crate::task::process::ProcessLoadMetadata {
        state: crate::task::process::ProcessLoadState::MappedStub,
        source_path: static_source_path(&prepared.image.source_path),
        entry_point: prepared.load_plan.entry_point,
        planned_pages: mapped.total_pages,
        region_count: mapped.regions.len(),
        stack_pages: prepared.load_plan.stack_pages,
        mapping_id: Some(mapped.id),
        copied_bytes: mapped.copied_bytes,
        zero_filled_bytes: mapped.zero_filled_bytes,
        executable_pages: mapped.executable_pages,
    };
    if let Some(pid) = crate::task::process::create_kernel_process_with_metadata(
        "image-mapped-stub",
        tick,
        credentials,
        metadata,
        load,
    ) {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Blocked);
    }
}

fn record_frame_backed_process(
    credentials: crate::security::Credentials,
    prepared: &PreparedProgramImage,
    backed: &crate::frame_backing::FrameBackedImage,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let metadata = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&prepared.program.source_path),
        format: prepared.image.format,
        entry_point: prepared.image.entry_point,
        segment_count: prepared.image.segments.len(),
        address_space_id: Some(backed.address_space_id),
        trust: prepared.image.trust,
        owner: credentials,
    };
    let load = crate::task::process::ProcessLoadMetadata {
        state: crate::task::process::ProcessLoadState::FrameBacked,
        source_path: static_source_path(&prepared.image.source_path),
        entry_point: prepared.load_plan.entry_point,
        planned_pages: backed.total_pages,
        region_count: backed.regions.len(),
        stack_pages: prepared.load_plan.stack_pages,
        mapping_id: Some(backed.mapping_id),
        copied_bytes: backed.copied_bytes,
        zero_filled_bytes: backed.zero_filled_bytes,
        executable_pages: backed.executable_pages,
    };
    if let Some(pid) = crate::task::process::create_kernel_process_with_metadata(
        "image-frame-backed",
        tick,
        credentials,
        metadata,
        load,
    ) {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Blocked);
    }
}

fn record_page_table_process(
    credentials: crate::security::Credentials,
    prepared: &PreparedProgramImage,
    backed: &crate::frame_backing::FrameBackedImage,
    page_table: &crate::user_memory::InactiveUserPageTable,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let metadata = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&prepared.program.source_path),
        format: prepared.image.format,
        entry_point: prepared.image.entry_point,
        segment_count: prepared.image.segments.len(),
        address_space_id: Some(backed.address_space_id),
        trust: prepared.image.trust,
        owner: credentials,
    };
    let load = crate::task::process::ProcessLoadMetadata {
        state: crate::task::process::ProcessLoadState::PageTableReady,
        source_path: static_source_path(&prepared.image.source_path),
        entry_point: prepared.load_plan.entry_point,
        planned_pages: page_table.mapped_pages,
        region_count: backed.regions.len(),
        stack_pages: prepared.load_plan.stack_pages,
        mapping_id: Some(backed.mapping_id),
        copied_bytes: backed.copied_bytes,
        zero_filled_bytes: backed.zero_filled_bytes,
        executable_pages: page_table.executable_pages,
    };
    if let Some(pid) = crate::task::process::create_kernel_process_with_metadata(
        "image-page-table",
        tick,
        credentials,
        metadata,
        load,
    ) {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Blocked);
    }
}

fn record_user_context_process(
    credentials: crate::security::Credentials,
    prepared: &PreparedProgramImage,
    backed: &crate::frame_backing::FrameBackedImage,
    context: &crate::user_context::UserContextDescriptor,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let metadata = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&prepared.program.source_path),
        format: prepared.image.format,
        entry_point: context.entry.rip,
        segment_count: prepared.image.segments.len(),
        address_space_id: Some(backed.address_space_id),
        trust: prepared.image.trust,
        owner: credentials,
    };
    let load = crate::task::process::ProcessLoadMetadata {
        state: crate::task::process::ProcessLoadState::UserContextReady,
        source_path: static_source_path(&prepared.image.source_path),
        entry_point: context.entry.rip,
        planned_pages: backed.total_pages,
        region_count: backed.regions.len(),
        stack_pages: prepared.load_plan.stack_pages,
        mapping_id: Some(backed.mapping_id),
        copied_bytes: backed.copied_bytes,
        zero_filled_bytes: backed.zero_filled_bytes,
        executable_pages: backed.executable_pages,
    };
    if let Some(pid) = crate::task::process::create_kernel_process_with_metadata(
        "image-user-context",
        tick,
        credentials,
        metadata,
        load,
    ) {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Blocked);
    }
}

fn record_ring3_trap_process(
    credentials: crate::security::Credentials,
    prepared: &PreparedProgramImage,
    backed: &crate::frame_backing::FrameBackedImage,
    result: &crate::ring3_trampoline::Ring3TrampolineResult,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let metadata = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&prepared.program.source_path),
        format: prepared.image.format,
        entry_point: result.entry_rip,
        segment_count: prepared.image.segments.len(),
        address_space_id: Some(backed.address_space_id),
        trust: prepared.image.trust,
        owner: credentials,
    };
    let load = crate::task::process::ProcessLoadMetadata {
        state: crate::task::process::ProcessLoadState::UserTrapped,
        source_path: static_source_path(&prepared.image.source_path),
        entry_point: result.entry_rip,
        planned_pages: backed.total_pages,
        region_count: backed.regions.len(),
        stack_pages: prepared.load_plan.stack_pages,
        mapping_id: Some(backed.mapping_id),
        copied_bytes: backed.copied_bytes,
        zero_filled_bytes: backed.zero_filled_bytes,
        executable_pages: backed.executable_pages,
    };
    if let Some(pid) = crate::task::process::create_kernel_process_with_metadata(
        "image-ring3-trap",
        tick,
        credentials,
        metadata,
        load,
    ) {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Blocked);
    }
}

fn record_user_syscall_process(
    credentials: crate::security::Credentials,
    prepared: &PreparedProgramImage,
    backed: &crate::frame_backing::FrameBackedImage,
    syscall_return: &crate::user_syscall::UserSyscallReturn,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let metadata = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&prepared.program.source_path),
        format: prepared.image.format,
        entry_point: prepared.image.entry_point,
        segment_count: prepared.image.segments.len(),
        address_space_id: Some(backed.address_space_id),
        trust: prepared.image.trust,
        owner: credentials,
    };
    let load = crate::task::process::ProcessLoadMetadata {
        state: crate::task::process::ProcessLoadState::UserSyscallReturned,
        source_path: static_source_path(&prepared.image.source_path),
        entry_point: syscall_return.return_value,
        planned_pages: backed.total_pages,
        region_count: backed.regions.len(),
        stack_pages: prepared.load_plan.stack_pages,
        mapping_id: Some(backed.mapping_id),
        copied_bytes: backed.copied_bytes,
        zero_filled_bytes: backed.zero_filled_bytes,
        executable_pages: backed.executable_pages,
    };
    if let Some(pid) = crate::task::process::create_kernel_process_with_metadata(
        "image-user-syscall",
        tick,
        credentials,
        metadata,
        load,
    ) {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Blocked);
    }
}

fn static_source_path(path: &str) -> &'static str {
    match path {
        "/bin/hello" => "/bin/hello",
        "/bin/hello.elf" => "/bin/hello.elf",
        "/bin/echo" => "/bin/echo",
        "/bin/time" => "/bin/time",
        "/bin/sysinfo" => "/bin/sysinfo",
        "/bin/fsinfo" => "/bin/fsinfo",
        _ => "<image>",
    }
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
        assert!(manifest.requires_execute);
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

    #[test_case]
    fn unsupported_trust_and_requirement_are_rejected() {
        assert_eq!(
            parse_manifest("ares-exec-v1\nname=x\nkind=builtin-alias\nentry=x\ntrust=unsigned"),
            Err(ProgramLoadError::UnsupportedTrust)
        );
        assert_eq!(
            parse_manifest("ares-exec-v1\nname=x\nkind=builtin-alias\nentry=x\nrequires=network"),
            Err(ProgramLoadError::UnsupportedRequirement)
        );
    }
}
