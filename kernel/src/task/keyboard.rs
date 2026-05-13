//! Asynchronous keyboard input handler.
//!
//! The keyboard IRQ handler writes raw scancodes into a lock-free
//! `ArrayQueue`.  The async `print_keypresses` future drains that queue,
//! translates scancodes to key events with the `pc-keyboard` crate, and
//! prints printable characters to the VGA console.

use alloc::{string::{String, ToString}, vec::Vec};
use crate::{print, println};
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

/// Maximum number of unprocessed scancodes to buffer.
const SCANCODE_QUEUE_SIZE: usize = 100;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(
            layouts::Us104Key,
            ScancodeSet1,
            HandleControl::Ignore,
        ));
    static ref CONSOLE_LINE: Mutex<String> = Mutex::new(String::new());
}

/// Initialise the keyboard scancode queue.
///
/// Safe to call multiple times; only the first call initialises storage.
pub fn init_scancode_queue() {
    let _ = SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(SCANCODE_QUEUE_SIZE));
}

/// Called by the keyboard IRQ handler to push a raw scancode into the queue.
///
/// This function is designed to be safe to call from an interrupt context:
/// it never blocks and never allocates.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            // Drop the scancode; the queue is full.
        } else {
            WAKER.wake();
        }
    } else {
        // Queue not yet initialised (very early boot); drop the scancode.
    }
}

/// An async `Stream` that yields raw scancodes from the IRQ handler.
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        init_scancode_queue();
        ScancodeStream { _private: () }
    }
}

impl Default for ScancodeStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialised");

        // Fast path: a scancode is already in the queue.
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // No scancode yet – register the waker so we are polled again when
        // the IRQ handler pushes a new scancode.
        WAKER.register(cx.waker());

        // Re-check after registering to avoid a TOCTOU race.
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

/// A future that reads keypresses from the keyboard and prints them.
///
/// This is the main keyboard task; spawn it with the executor at boot.
pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();

    while let Some(scancode) = scancodes.next().await {
        process_scancode(scancode, true);
    }
    println!("Keyboard stream ended.");
}

/// Poll queued scancodes and process keyboard-console commands.
///
/// This is used by preemption-mode context tasks where the async keyboard
/// task is not running.
pub fn poll_console_commands() {
    while let Some(scancode) = try_pop_scancode() {
        process_scancode(scancode, true);
    }
}

fn try_pop_scancode() -> Option<u8> {
    SCANCODE_QUEUE.try_get().ok().and_then(|queue| queue.pop())
}

fn process_scancode(scancode: u8, echo: bool) {
    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => handle_console_char(character, echo),
                DecodedKey::RawKey(key) => {
                    if echo {
                        print!("{:?}", key);
                    }
                }
            }
        }
    }
}

fn handle_console_char(character: char, echo: bool) {
    match character {
        '\n' | '\r' => {
            if echo {
                println!("");
            }
            let command = {
                let mut line = CONSOLE_LINE.lock();
                let command = line.trim().to_string();
                line.clear();
                command
            };

            if command.is_empty() {
                return;
            }

            execute_console_command(&command);
        }
        '\u{8}' | '\u{7f}' => {
            let mut line = CONSOLE_LINE.lock();
            if !line.is_empty() {
                line.pop();
                if echo {
                    print!("\u{8} \u{8}");
                }
            }
        }
        c => {
            let mut line = CONSOLE_LINE.lock();
            line.push(c);
            if echo {
                print!("{}", c);
            }
        }
    }
}

fn execute_console_command(command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts.as_slice() {
        ["help"] => {
            println!("Console commands:");
            println!("  help");
            println!("  ps");
            println!("  kill <pid>");
            println!("  metrics");
            println!("  whoami");
            println!("  su <admin|user|guest>");
            println!("  run <echo|time|sysinfo|fsinfo> [args...]");
            println!("  programs");
            println!("  bin list");
            println!("  bin info <program>");
            println!("  bin validate <program>");
            println!("  bin prepare <program>");
            println!("  bin map <program>");
            println!("  bin back <program>");
            println!("  bin pagetable <program>");
            println!("  bin userctx <program>");
            println!("  bin plans");
            println!("  bin mappings");
            println!("  frames");
            println!("  ls");
            println!("  cat <path>");
            println!("  touch <path>");
            println!("  write <path> <text>");
            println!("  rm <path>");
            println!("  stat <path>");
            println!("  chmod +x|-x <path>");
            println!("  mount");
            println!("  format");
            println!("  fsinfo");
            println!("  devices");
            println!("  blk list");
            println!("  blk info <id>");
            println!("  mount <block-id>");
            println!("  sched show");
            println!("  sched quantum <ticks>");
            println!("  sched fairness <ticks>");
            println!("  sched maxproc <count>");
        }
        ["ps"] => {
            let entries = crate::task::process::get_all_processes_with_details();
            if entries.is_empty() {
                println!("No processes registered");
            } else {
                println!("PID  STATE       CPU_TICKS  OWNER      IMAGE          LOAD       NAME");
                for (pid, name, state, ticks, owner, image, load) in entries {
                    let image_source = image
                        .as_ref()
                        .map(|image| image.source_path)
                        .unwrap_or("-");
                    let load_state = load
                        .as_ref()
                        .map(|load| match load.state {
                            crate::task::process::ProcessLoadState::Prepared => "prepared",
                            crate::task::process::ProcessLoadState::Rejected => "rejected",
                            crate::task::process::ProcessLoadState::ExecutionBlocked => "blocked",
                            crate::task::process::ProcessLoadState::MappedStub => "mapped",
                            crate::task::process::ProcessLoadState::FrameBacked => "backed",
                            crate::task::process::ProcessLoadState::PageTableReady => "ptable",
                            crate::task::process::ProcessLoadState::UserContextReady => "uctx",
                        })
                        .unwrap_or("-");
                    println!(
                        "{:<4} {:<11?} {:<9} {:<10} {:<14} {:<10} {}",
                        pid.as_u64(),
                        state,
                        ticks,
                        owner.role.name(),
                        image_source,
                        load_state,
                        name
                    );
                }
            }
        }
        ["kill", pid] => match parse_pid(pid) {
            Ok(raw_pid) => {
                let pid = crate::task::process::ProcessId::from_raw(raw_pid);
                if crate::task::process::terminate_process_checked(
                    crate::security::current_credentials(),
                    pid,
                    0,
                ) {
                    println!("Terminated PID {}", raw_pid);
                } else {
                    println!("PID {} not found or permission denied", raw_pid);
                }
            }
            Err(err) => println!("Invalid pid ({}): {}", err, pid),
        },
        ["whoami"] => {
            let credentials = crate::security::current_credentials();
            println!(
                "user={} role={}",
                credentials.user.as_u64(),
                credentials.role.name()
            );
        }
        ["su", role] => match *role {
            "admin" => {
                crate::security::set_current_credentials(crate::security::Credentials::admin());
                println!("Switched to admin");
            }
            "user" => {
                crate::security::set_current_credentials(crate::security::Credentials::shell_user());
                println!("Switched to user");
            }
            "guest" => {
                crate::security::set_current_credentials(crate::security::Credentials::guest());
                println!("Switched to guest");
            }
            _ => println!("Unknown role: {}", role),
        },
        ["metrics"] => {
            let scheduler = crate::task::scheduler::stats();
            let (creates, terms, preemptions, fairness_violations) =
                crate::performance::process_metrics::ProcessMetricsGlobal::global_snapshot();
            println!(
                "Metrics: ticks={}, req={}, points={}, preemptions={}, creates={}, terms={}, fairness_violations={}",
                scheduler.timer_ticks,
                scheduler.reschedule_requests,
                scheduler.reschedule_points,
                preemptions,
                creates,
                terms,
                fairness_violations
            );
        }
        ["run", program, args @ ..] => match crate::task::userspace::run_program(program, args) {
            Ok(output) => println!("{}", output),
            Err(err) => println!("run error: {}", err),
        },
        ["programs"] | ["bin", "list"] => {
            let programs = crate::task::program_loader::discover_programs();
            if programs.is_empty() {
                println!("No stored programs discovered");
            } else {
                for program in programs {
                    let marker = match program.kind {
                        crate::task::program_loader::ProgramKind::BuiltinAlias => "builtin",
                        crate::task::program_loader::ProgramKind::Elf64Image => "elf64-image",
                    };
                    println!(
                        "{} [{}] -> {} ({})",
                        program.name, marker, program.entry, program.source_path
                    );
                }
            }
            let status = crate::task::program_loader::status();
            println!(
                "Program loader: programs={}, images={}/{}, invalid_images={}, prepared={}, planned_pages={}, mapped={}, mapped_pages={}, launches={}, failed_launches={}",
                status.program_count,
                status.valid_image_count,
                status.image_count,
                status.invalid_image_count,
                status.prepared_image_count,
                status.total_planned_pages,
                status.mapped_image_count,
                status.total_mapped_pages,
                status.launch_count,
                status.failed_launch_count
            );
        }
        ["bin", "info", program] => match crate::task::program_loader::program_info(program) {
            Ok(info) => {
                let planned = info
                    .image
                    .as_ref()
                    .and_then(|image| crate::load_plan::build_load_plan(image).ok());
                println!(
                    "Program {}: path={}, kind={:?}, entry={}, image={:?}, segments={}, planned_pages={}, planned_regions={}, trust={:?}, exec_supported={}, description={}",
                    info.name,
                    info.source_path,
                    info.kind,
                    info.entry,
                    info.image_path,
                    info.image.as_ref().map(|image| image.segments.len()).unwrap_or(0),
                    planned.as_ref().map(|plan| plan.total_pages).unwrap_or(0),
                    planned.as_ref().map(|plan| plan.regions.len()).unwrap_or(0),
                    info.trust,
                    info.kind == crate::task::program_loader::ProgramKind::BuiltinAlias,
                    info.description
                );
            }
            Err(err) => println!("program info error: {:?}", err),
        },
        ["bin", "validate", program] => match crate::task::program_loader::validate_program_image(
            crate::security::current_credentials(),
            program,
        ) {
            Ok(image) => println!(
                "Program {} image valid: format={:?}, entry=0x{:x}, segments={}, source={}",
                image.name,
                image.format,
                image.entry_point,
                image.segments.len(),
                image.source_path
            ),
            Err(err) => println!("program validate error: {:?}", err),
        },
        ["bin", "prepare", program] => match crate::task::program_loader::prepare_program_image(
            crate::security::current_credentials(),
            program,
        ) {
            Ok(prepared) => println!(
                "Prepared {}: entry=0x{:x}, regions={}, pages={}, stack_pages={}",
                prepared.image.name,
                prepared.load_plan.entry_point,
                prepared.load_plan.regions.len(),
                prepared.load_plan.total_pages,
                prepared.load_plan.stack_pages
            ),
            Err(err) => println!("program prepare error: {:?}", err),
        },
        ["bin", "map", program] => match crate::task::program_loader::map_prepared_program(
            crate::security::current_credentials(),
            program,
        ) {
            Ok(mapped) => println!(
                "Mapped {}: id={}, pages={}, copied={}, zeroed={}, state={:?}",
                mapped.mapped.image_name,
                mapped.mapped.id.as_u64(),
                mapped.mapped.total_pages,
                mapped.mapped.copied_bytes,
                mapped.mapped.zero_filled_bytes,
                mapped.mapped.state
            ),
            Err(err) => println!("program map error: {:?}", err),
        },
        ["bin", "back", program] => match crate::task::program_loader::back_mapped_program(
            crate::security::current_credentials(),
            program,
        ) {
            Ok(backed) => println!(
                "Frame-backed {}: mapping={}, pages={}, copied={}, zeroed={}, state={:?}",
                backed.backed.image_name,
                backed.backed.mapping_id.as_u64(),
                backed.backed.total_pages,
                backed.backed.copied_bytes,
                backed.backed.zero_filled_bytes,
                backed.backed.state
            ),
            Err(err) => println!("program frame-back error: {:?}", err),
        },
        ["bin", "pagetable", program] => match crate::task::program_loader::build_user_page_table(
            crate::security::current_credentials(),
            program,
        ) {
            Ok(table) => println!(
                "Inactive page table {}: asid={}, pages={}, exec={}, writable={}, readonly={}, cr3_ready={}",
                table.page_table.id.as_u64(),
                table.page_table.address_space_id.as_u64(),
                table.page_table.mapped_pages,
                table.page_table.executable_pages,
                table.page_table.writable_pages,
                table.page_table.read_only_pages,
                table.page_table.cr3_switch_ready
            ),
            Err(err) => println!("program page-table error: {:?}", err),
        },
        ["bin", "userctx", program] => match crate::task::program_loader::prepare_user_context(
            crate::security::current_credentials(),
            program,
        ) {
            Ok(userctx) => println!(
                "User context: page_table={}, rip=0x{:x}, rsp=0x{:x}, cs={}, ss={}, ring3_entered={}",
                userctx.context.page_table_id.as_u64(),
                userctx.context.entry.rip,
                userctx.context.entry.rsp,
                userctx.context.entry.code_selector,
                userctx.context.entry.stack_selector,
                userctx.context.ring3_entered
            ),
            Err(err) => println!("program user-context error: {:?}", err),
        },
        ["bin", "plans"] | ["loadplans"] => {
            let status = crate::task::program_loader::status();
            println!(
                "Load plans: prepared={}, rejected={}, planned_pages={}, mapped={}, mapped_pages={}, backed={}, backed_pages={}, page_tables={}, ptable_pages={}, user_contexts={}, exec_blocked={}",
                status.prepared_image_count,
                status.rejected_load_plan_count,
                status.total_planned_pages,
                status.mapped_image_count,
                status.total_mapped_pages,
                status.frame_backed_image_count,
                status.total_frame_backed_pages,
                status.user_page_table_count,
                status.total_user_page_table_pages,
                status.user_context_count,
                status.unsupported_execution_count
            );
        }
        ["bin", "mappings"] => {
            for mapping in crate::mapping_stub::list_mappings() {
                println!(
                    "Mapping {}: image={}, asid={}, pages={}, copied={}, zeroed={}, state={:?}",
                    mapping.id.as_u64(),
                    mapping.image_name,
                    mapping.address_space_id.as_u64(),
                    mapping.total_pages,
                    mapping.copied_bytes,
                    mapping.zero_filled_bytes,
                    mapping.state
                );
            }
        }
        ["frames"] => {
            let status = crate::frame_ownership::status();
            println!(
                "Frames: initialized={}, tracked={}, available={}, allocated={}, allocations={}, releases={}, failures={}",
                status.initialized,
                status.tracked_frames,
                status.available_frames,
                status.allocated_frames,
                status.allocation_count,
                status.release_count,
                status.failed_allocation_count
            );
        }
        ["ls"] => match crate::storage::list_files() {
            Ok(files) => {
                for file in files {
                    println!("{}", file);
                }
            }
            Err(err) => println!("ls error: {}", err),
        },
        ["cat", path] => match crate::storage::read_file_checked(
            crate::security::current_credentials(),
            path,
        ) {
            Ok(Some(contents)) => println!("{}", contents),
            Ok(None) => println!("No such file: {}", path),
            Err(err) => println!("cat error: {}", err),
        },
        ["touch", path] => match crate::storage::create_file_checked(
            crate::security::current_credentials(),
            path,
        ) {
            Ok(()) => println!("Created {}", path),
            Err(crate::storage::StorageError::AlreadyExists) => println!("File already exists: {}", path),
            Err(err) => println!("touch error: {}", err),
        },
        ["write", path, contents @ ..] if !contents.is_empty() => {
            let text = join_parts(contents);
            match crate::storage::write_file_checked(
                crate::security::current_credentials(),
                path,
                &text,
            ) {
                Ok(()) => println!("Wrote {}", path),
                Err(err) => println!("write error: {}", err),
            }
        }
        ["write", ..] => println!("Usage: write <path> <text>"),
        ["rm", path] => match crate::storage::delete_file_checked(
            crate::security::current_credentials(),
            path,
        ) {
            Ok(()) => println!("Removed {}", path),
            Err(err) => println!("rm error: {}", err),
        },
        ["stat", path] => match crate::storage::stat_file(path) {
            Ok(Some(metadata)) => println!(
                "File {}: owner={}, mode={:03b}, len={}",
                metadata.path,
                metadata.owner.as_u64(),
                metadata.mode.bits(),
                metadata.len
            ),
            Ok(None) => println!("No such file: {}", path),
            Err(err) => println!("stat error: {}", err),
        },
        ["chmod", flag, path] => match *flag {
            "+x" => match crate::storage::chmod_execute_checked(
                crate::security::current_credentials(),
                path,
                true,
            ) {
                Ok(()) => println!("Enabled execute on {}", path),
                Err(err) => println!("chmod error: {}", err),
            },
            "-x" => match crate::storage::chmod_execute_checked(
                crate::security::current_credentials(),
                path,
                false,
            ) {
                Ok(()) => println!("Disabled execute on {}", path),
                Err(err) => println!("chmod error: {}", err),
            },
            _ => println!("Usage: chmod +x|-x <path>"),
        },
        ["mount"] => match crate::storage::remount() {
            Ok(()) => println!("Storage mounted"),
            Err(err) => println!("mount error: {}", err),
        },
        ["mount", block_id] => match parse_block_id(block_id) {
            Ok(id) => match crate::storage::mount_block_device(id) {
                Ok(()) => println!("Mounted block device {}", id),
                Err(err) => println!("mount error: {}", err),
            },
            Err(err) => println!("Invalid block id ({}): {}", err, block_id),
        },
        ["format"] => match crate::storage::format() {
            Ok(()) => println!("Storage formatted"),
            Err(err) => println!("format error: {}", err),
        },
        ["fsinfo"] => match crate::storage::info() {
            Ok(info) => println!(
                "FS: mounted={}, files={}/{}, free_slots={}, capacity_bytes={}, max_file_size={}, backend={}, driver_backed={}",
                info.mounted,
                info.file_count,
                info.max_files,
                info.free_slots,
                info.capacity_bytes,
                info.max_file_size,
                info.backend_name,
                info.driver_backed
            ),
            Err(err) => println!("fsinfo error: {}", err),
        },
        ["devices"] => {
            let summary = crate::device::summary();
            println!(
                "Devices: total={}, pci={}, block={}, storage={}",
                summary.total, summary.pci, summary.block, summary.storage
            );
            for device in crate::device::list_devices() {
                println!(
                    "  id={} kind={:?} state={:?} name={} vendor={:?} device={:?} class={:?} subclass={:?}",
                    device.id.as_u64(),
                    device.kind,
                    device.state,
                    device.name,
                    device.vendor_id,
                    device.device_id,
                    device.class_code,
                    device.subclass
                );
            }
        }
        ["blk", "list"] => {
            for device in crate::block::list_block_devices() {
                println!(
                    "  id={} name={} backend={:?} sectors={} sector_size={} readonly={} driver_backed={}",
                    device.id.as_u64(),
                    device.name,
                    device.backend,
                    device.sector_count,
                    device.sector_size,
                    device.read_only,
                    device.driver_backed
                );
            }
        }
        ["blk", "info", block_id] => match parse_block_id(block_id) {
            Ok(id) => {
                let found = crate::block::list_block_devices()
                    .into_iter()
                    .find(|device| device.id.as_u64() == id);
                match found {
                    Some(device) => println!(
                        "Block {}: name={}, backend={:?}, sectors={}, sector_size={}, readonly={}, driver_backed={}",
                        id,
                        device.name,
                        device.backend,
                        device.sector_count,
                        device.sector_size,
                        device.read_only,
                        device.driver_backed
                    ),
                    None => println!("Block device {} not found", id),
                }
            }
            Err(err) => println!("Invalid block id ({}): {}", err, block_id),
        },
        ["sched", "show"] => {
            let config = crate::task::scheduler::runtime_config();
            println!(
                "Scheduler config: quantum_ticks={}, fairness_check_ticks={}, max_processes={}",
                config.quantum_ticks, config.fairness_check_interval_ticks, config.max_processes
            );
            crate::serial_println!(
                "Scheduler config: quantum_ticks={}, fairness_check_ticks={}, max_processes={}",
                config.quantum_ticks, config.fairness_check_interval_ticks, config.max_processes
            );
        }
        ["sched", "quantum", value] => match value.parse::<u64>() {
            Ok(ticks) => {
                let mut config = crate::task::scheduler::runtime_config();
                config.quantum_ticks = ticks;
                match crate::task::scheduler::apply_runtime_config(config) {
                    Ok(_) => println!("Updated scheduler quantum to {} ticks", config.quantum_ticks),
                    Err(err) => println!("Rejected scheduler update: {:?}", err),
                }
            }
            Err(_) => println!("Invalid quantum value: {}", value),
        },
        ["sched", "fairness", value] => match value.parse::<u64>() {
            Ok(ticks) => {
                let mut config = crate::task::scheduler::runtime_config();
                config.fairness_check_interval_ticks = ticks;
                match crate::task::scheduler::apply_runtime_config(config) {
                    Ok(_) => println!(
                        "Updated fairness check interval to {} ticks",
                        config.fairness_check_interval_ticks
                    ),
                    Err(err) => println!("Rejected scheduler update: {:?}", err),
                }
            }
            Err(_) => println!("Invalid fairness value: {}", value),
        },
        ["sched", "maxproc", value] => match value.parse::<usize>() {
            Ok(max_proc) => {
                let mut config = crate::task::scheduler::runtime_config();
                config.max_processes = max_proc;
                match crate::task::scheduler::apply_runtime_config(config) {
                    Ok(_) => println!("Updated max processes to {}", config.max_processes),
                    Err(err) => println!("Rejected scheduler update: {:?}", err),
                }
            }
            Err(_) => println!("Invalid maxproc value: {}", value),
        },
        _ => {
            println!("Unknown command: {}", command);
            println!("Type 'help' for available commands");
        }
    }
}

fn parse_pid(value: &str) -> Result<u64, &'static str> {
    let pid = value.parse::<u64>().map_err(|_| "not-a-number")?;
    if pid == 0 {
        return Err("reserved-pid");
    }
    Ok(pid)
}

fn parse_block_id(value: &str) -> Result<u64, &'static str> {
    let id = value.parse::<u64>().map_err(|_| "not-a-number")?;
    if id == 0 {
        return Err("reserved-id");
    }
    Ok(id)
}

fn join_parts(parts: &[&str]) -> String {
    let mut out = String::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        out.push_str(part);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::parse_pid;

    #[test_case]
    fn parse_pid_rejects_reserved_pid_zero() {
        assert_eq!(parse_pid("0"), Err("reserved-pid"));
    }

    #[test_case]
    fn parse_pid_rejects_non_numeric() {
        assert_eq!(parse_pid("abc"), Err("not-a-number"));
    }

    #[test_case]
    fn parse_pid_accepts_positive_ids() {
        assert_eq!(parse_pid("42"), Ok(42));
    }

    #[test_case]
    fn parse_block_id_rejects_zero() {
        assert_eq!(super::parse_block_id("0"), Err("reserved-id"));
    }

    #[test_case]
    fn join_parts_preserves_spaces_between_words() {
        assert_eq!(super::join_parts(&["hello", "phase", "7"]), "hello phase 7");
    }

    #[test_case]
    fn scheduler_console_updates_apply() {
        let baseline = crate::task::scheduler::SchedulerRuntimeConfig {
            quantum_ticks: 5,
            fairness_check_interval_ticks: 10,
            max_processes: 256,
        };
        let _ = crate::task::scheduler::apply_runtime_config(baseline);

        super::execute_console_command("sched quantum 7");
        super::execute_console_command("sched fairness 13");
        super::execute_console_command("sched maxproc 321");

        let config = crate::task::scheduler::runtime_config();
        assert_eq!(config.quantum_ticks, 7);
        assert_eq!(config.fairness_check_interval_ticks, 13);
        assert_eq!(config.max_processes, 321);
    }

    #[test_case]
    fn scheduler_console_invalid_update_rolls_back() {
        let baseline = crate::task::scheduler::SchedulerRuntimeConfig {
            quantum_ticks: 6,
            fairness_check_interval_ticks: 12,
            max_processes: 256,
        };
        let _ = crate::task::scheduler::apply_runtime_config(baseline);

        super::execute_console_command("sched quantum 0");
        super::execute_console_command("sched maxproc 0");

        let config = crate::task::scheduler::runtime_config();
        assert_eq!(config, baseline);
    }
}
