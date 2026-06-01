use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write as _},
    sync::OnceLock,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const DIRECT_LOG_FILE_ENV: &str = "WRAPPER_DIRECT_LOG_FILE";
const DIRECT_LOG_RUN_ID_ENV: &str = "WRAPPER_DIRECT_LOG_RUN_ID";
const DIRECT_LOG_COMMAND_ENV: &str = "WRAPPER_DIRECT_LOG_COMMAND";
const DIRECT_LOG_IDENTIFIER_ENV: &str = "WRAPPER_DIRECT_LOG_IDENTIFIER";
const DIRECT_LOG_BACKEND_ENV: &str = "WRAPPER_DIRECT_LOG_BACKEND";
const DIRECT_LOG_HOST_ENV: &str = "WRAPPER_DIRECT_LOG_HOST";
const DIRECT_LOG_MODE_ENV: &str = "WRAPPER_DIRECT_LOG_MODE";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DirectLogMode {
    Detailed,
    Efficient,
}

fn direct_log_mode() -> DirectLogMode {
    static LOG_MODE: OnceLock<DirectLogMode> = OnceLock::new();
    *LOG_MODE.get_or_init(|| match std::env::var(DIRECT_LOG_MODE_ENV).ok().as_deref() {
        Some("efficient") => DirectLogMode::Efficient,
        _ => DirectLogMode::Detailed,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct MemorySnapshot {
    rss_kb: Option<u64>,
    hwm_kb: Option<u64>,
    vmsize_kb: Option<u64>,
}

impl MemorySnapshot {
    fn capture() -> Self {
        let Ok(file) = std::fs::File::open("/proc/self/status") else {
            return Self::default();
        };
        let reader = BufReader::new(file);
        let mut snapshot = Self::default();

        for line in reader.lines().map_while(Result::ok) {
            if let Some(value) = parse_status_kb_field(&line, "VmRSS:") {
                snapshot.rss_kb = Some(value);
            } else if let Some(value) = parse_status_kb_field(&line, "VmHWM:") {
                snapshot.hwm_kb = Some(value);
            } else if let Some(value) = parse_status_kb_field(&line, "VmSize:") {
                snapshot.vmsize_kb = Some(value);
            }
        }

        snapshot
    }
}

fn parse_status_kb_field(line: &str, label: &str) -> Option<u64> {
    let rest = line.strip_prefix(label)?.trim();
    let numeric = rest.split_whitespace().next()?;
    numeric.parse::<u64>().ok()
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn append_log_line(line: &str) {
    if let Ok(path) = std::env::var(DIRECT_LOG_FILE_ENV) {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn append_field(line: &mut String, key: &str, value: impl AsRef<str>) {
    let value = value.as_ref();
    if value.is_empty() {
        return;
    }
    line.push(' ');
    line.push_str(key);
    line.push('=');
    line.push_str(value);
}

fn append_optional_context_field(line: &mut String, key: &str, env_key: &str) {
    if let Ok(value) = std::env::var(env_key) {
        if !value.is_empty() {
            append_field(line, key, value);
        }
    }
}

pub(crate) fn log_event(phase: &str, step: &str, event: &str, extra: &str) {
    if event == "iter" && direct_log_mode() == DirectLogMode::Efficient {
        return;
    }
    let snapshot = MemorySnapshot::capture();
    let mut line = format!(
        "ts_ms={} level=INFO run_id={} pid={} phase={} step={} event={}",
        now_millis(),
        std::env::var(DIRECT_LOG_RUN_ID_ENV)
            .unwrap_or_else(|_| format!("pid{}", std::process::id())),
        std::process::id(),
        phase,
        step,
        event
    );

    append_optional_context_field(&mut line, "command", DIRECT_LOG_COMMAND_ENV);
    append_optional_context_field(&mut line, "identifier", DIRECT_LOG_IDENTIFIER_ENV);
    append_optional_context_field(&mut line, "backend", DIRECT_LOG_BACKEND_ENV);
    append_optional_context_field(&mut line, "host", DIRECT_LOG_HOST_ENV);

    if let Some(rss_kb) = snapshot.rss_kb {
        append_field(&mut line, "rss_kb", rss_kb.to_string());
    }
    if let Some(hwm_kb) = snapshot.hwm_kb {
        append_field(&mut line, "hwm_kb", hwm_kb.to_string());
    }
    if let Some(vmsize_kb) = snapshot.vmsize_kb {
        append_field(&mut line, "vmsize_kb", vmsize_kb.to_string());
    }
    if !extra.is_empty() {
        line.push(' ');
        line.push_str(extra);
    }

    append_log_line(&line);
}

pub(crate) fn log_elapsed(phase: &str, step: &str, started_at: Instant, extra: &str) {
    let mut merged_extra = format!("elapsed_ms={}", started_at.elapsed().as_millis());
    if !extra.is_empty() {
        merged_extra.push(' ');
        merged_extra.push_str(extra);
    }
    log_event(phase, step, "end", &merged_extra);
}
