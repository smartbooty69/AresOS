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
            println!("  run <echo|time|sysinfo|fsinfo> [args...]");
            println!("  ls");
            println!("  cat <path>");
            println!("  touch <path>");
            println!("  write <path> <text>");
            println!("  rm <path>");
            println!("  mount");
            println!("  format");
            println!("  fsinfo");
            println!("  sched show");
            println!("  sched quantum <ticks>");
            println!("  sched fairness <ticks>");
            println!("  sched maxproc <count>");
        }
        ["ps"] => {
            let entries = crate::task::process::get_all_processes();
            if entries.is_empty() {
                println!("No processes registered");
            } else {
                println!("PID  STATE       CPU_TICKS  NAME");
                for (pid, name, state, ticks) in entries {
                    println!("{:<4} {:<11?} {:<9} {}", pid.as_u64(), state, ticks, name);
                }
            }
        }
        ["kill", pid] => match parse_pid(pid) {
            Ok(raw_pid) => {
                let pid = crate::task::process::ProcessId::from_raw(raw_pid);
                if crate::task::process::terminate_process(pid, 0) {
                    println!("Terminated PID {}", raw_pid);
                } else {
                    println!("PID {} not found", raw_pid);
                }
            }
            Err(err) => println!("Invalid pid ({}): {}", err, pid),
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
        ["ls"] => match crate::storage::list_files() {
            Ok(files) => {
                for file in files {
                    println!("{}", file);
                }
            }
            Err(err) => println!("ls error: {}", err),
        },
        ["cat", path] => match crate::storage::read_file(path) {
            Ok(Some(contents)) => println!("{}", contents),
            Ok(None) => println!("No such file: {}", path),
            Err(err) => println!("cat error: {}", err),
        },
        ["touch", path] => match crate::storage::create_file(path) {
            Ok(()) => println!("Created {}", path),
            Err(crate::storage::StorageError::AlreadyExists) => println!("File already exists: {}", path),
            Err(err) => println!("touch error: {}", err),
        },
        ["write", path, contents @ ..] if !contents.is_empty() => {
            let text = join_parts(contents);
            match crate::storage::write_file(path, &text) {
                Ok(()) => println!("Wrote {}", path),
                Err(err) => println!("write error: {}", err),
            }
        }
        ["write", ..] => println!("Usage: write <path> <text>"),
        ["rm", path] => match crate::storage::delete_file(path) {
            Ok(()) => println!("Removed {}", path),
            Err(err) => println!("rm error: {}", err),
        },
        ["mount"] => match crate::storage::remount() {
            Ok(()) => println!("Storage mounted"),
            Err(err) => println!("mount error: {}", err),
        },
        ["format"] => match crate::storage::format() {
            Ok(()) => println!("Storage formatted"),
            Err(err) => println!("format error: {}", err),
        },
        ["fsinfo"] => match crate::storage::info() {
            Ok(info) => println!(
                "FS: mounted={}, files={}/{}, free_slots={}, capacity_bytes={}, max_file_size={}",
                info.mounted,
                info.file_count,
                info.max_files,
                info.free_slots,
                info.capacity_bytes,
                info.max_file_size
            ),
            Err(err) => println!("fsinfo error: {}", err),
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
