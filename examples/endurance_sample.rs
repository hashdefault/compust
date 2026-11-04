//! Daily-use endurance sampler and seven-day summary check for roadmap step 4.
//!
//! `sample` queries the running X session at a fixed interval without selecting events or
//! mutating server state, appending one flushed CSV row per sample so an interrupted week-long
//! run keeps earlier measurements. `summary` evaluates a recorded CSV against the four release
//! criteria: seven-day span, no compositor crash or restart, no repeated Present timeouts, and
//! last-day idle resident memory within 5% of the first day's after warmup.

use anyhow::{Context, Result, bail, ensure};
use clap::{Args as ClapArgs, Parser, Subcommand};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::ExitCode,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use x11rb::{
    NONE,
    connection::Connection,
    errors::ReplyError,
    protocol::{
        res::{ClientIdMask, ClientIdSpec, ConnectionExt as _, ResourceIdSpec},
        screensaver::ConnectionExt as _,
        xproto::{AtomEnum, ConnectionExt as _},
    },
    rust_connection::RustConnection,
};

const SECONDS_PER_DAY: u64 = 86_400;
const REQUIRED_DAYS: u64 = 7;
const MIN_MEMORY_DAYS: u64 = 2;
const PRESENT_TIMEOUT_WARNING: &str = "Present did not finish a submission";

const CSV_HEADER: &str = "unix_time,utc,status,pid,start_time,rss_kib,pixmap_bytes,resources_total,PIXMAP,PICTURE,WINDOW,DAMAGE,REGION,GC,FENCE,resources,idle_ms,present_timeouts,server_rss_kib,server_session";

#[derive(Parser)]
#[command(about = "Sample and summarize a daily-use Compust endurance run")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Periodically sample Compust's memory and X server resources into a CSV file.
    Sample(SampleArgs),
    /// Evaluate a sampled CSV file against the seven-day endurance criteria.
    Summary(SummaryArgs),
}

#[derive(ClapArgs)]
struct SampleArgs {
    /// X11 display to query, such as :0.
    #[arg(long)]
    display: String,
    /// CSV file to create or append to.
    #[arg(long)]
    output: PathBuf,
    /// Seconds between samples.
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..), default_value_t = 600)]
    interval: u64,
    /// Compust stderr log file to scan for Present timeout warnings.
    #[arg(long)]
    log: Option<PathBuf>,
    /// X server PID whose resident memory to record. Its start, with the boot's ID, also names
    /// the session, so that `summary` tells a compositor that crashed from one that a logout or
    /// a shutdown ended.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    server_pid: Option<u32>,
    /// Take one sample and exit.
    #[arg(long)]
    once: bool,
}

#[derive(ClapArgs)]
struct SummaryArgs {
    /// Sampled CSV file to evaluate.
    #[arg(long)]
    input: PathBuf,
    /// Initial minutes ignored when comparing first-day and last-day idle memory.
    #[arg(long, default_value_t = 60)]
    warmup_minutes: u64,
    /// Minimum milliseconds without user input for a sample to count as idle.
    #[arg(long, default_value_t = 300_000)]
    idle_ms: u64,
}

fn main() -> Result<ExitCode> {
    let args = Args::parse();
    match args.command {
        Command::Sample(sample_args) => {
            run_sample(&sample_args)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Summary(summary_args) => run_summary(&summary_args),
    }
}

/// Holds the read-only X11 connection and cached atom names across samples so each interval
/// sends only the queries needed for one row.
struct SessionSampler {
    conn: RustConnection,
    root: u32,
    selection_name: String,
    selection_atom: u32,
    atom_names: BTreeMap<u32, String>,
}

/// One sample of the compositor's process and `XRes` state when it owns `_NET_WM_CM_S<screen>`.
struct CompositorMetrics {
    pid: u32,
    start_time: u64,
    rss_kib: u64,
    pixmap_bytes: u64,
    resources_total: u64,
    pixmap: u32,
    picture: u32,
    window: u32,
    damage: u32,
    region: u32,
    gc: u32,
    fence: u32,
    resources: String,
}

impl SessionSampler {
    fn connect(display: &str) -> Result<Self> {
        let (conn, screen_number) =
            x11rb::connect(Some(display)).with_context(|| format!("connecting to {display}"))?;
        let screen = conn
            .setup()
            .roots
            .get(screen_number)
            .context("missing X11 screen")?;
        let root = screen.root;
        let res_version = conn.res_query_version(1, 2)?.reply()?;
        ensure!(
            (res_version.server_major, res_version.server_minor) >= (1, 2),
            "resource accounting requires XRes 1.2"
        );
        conn.screensaver_query_version(1, 0)?.reply()?;
        let selection_name = format!("_NET_WM_CM_S{screen_number}");
        Ok(Self {
            conn,
            root,
            selection_name,
            selection_atom: NONE,
            atom_names: BTreeMap::new(),
        })
    }

    /// Milliseconds since the last user input on the screen, queried through MIT-SCREEN-SAVER.
    fn idle_ms(&self) -> Result<u64> {
        let info = self.conn.screensaver_query_info(self.root)?.reply()?;
        Ok(u64::from(info.ms_since_user_input))
    }

    /// Sample the active compositor if one owns `_NET_WM_CM_S<screen>`.
    ///
    /// Returns `Ok(None)` when no compositor owns the selection or when the owner disappears
    /// while its `XRes` or `/proc` state is being read; returns `Err` only when the X connection
    /// itself fails.
    fn sample_compositor(&mut self) -> Result<Option<CompositorMetrics>> {
        if self.selection_atom == NONE {
            // `only_if_exists = true` avoids creating the atom before a compositor has run.
            self.selection_atom = self
                .conn
                .intern_atom(true, self.selection_name.as_bytes())?
                .reply()?
                .atom;
            if self.selection_atom == NONE {
                return Ok(None);
            }
        }
        let owner = self
            .conn
            .get_selection_owner(self.selection_atom)?
            .reply()?
            .owner;
        if owner == NONE {
            return Ok(None);
        }
        let ids_cookie = self.conn.res_query_client_ids(&[ClientIdSpec {
            client: owner,
            mask: ClientIdMask::LOCAL_CLIENT_PID,
        }])?;
        let resources_cookie = self.conn.res_query_client_resources(owner)?;
        let bytes_cookie = self.conn.res_query_resource_bytes(
            owner,
            &[ResourceIdSpec {
                resource: NONE,
                type_: NONE,
            }],
        )?;

        let Some(ids_reply) = non_fatal_x11(ids_cookie.reply())? else {
            return Ok(None);
        };
        let Some(resources_reply) = non_fatal_x11(resources_cookie.reply())? else {
            return Ok(None);
        };
        let Some(bytes_reply) = non_fatal_x11(bytes_cookie.reply())? else {
            return Ok(None);
        };

        let Some(pid) = ids_reply
            .ids
            .iter()
            .find(|id| id.spec.mask.contains(ClientIdMask::LOCAL_CLIENT_PID))
            .and_then(|id| id.value.first().copied())
            .filter(|&pid| pid > 0)
        else {
            return Ok(None);
        };

        let (Ok(start_time), Ok(rss_kib)) = (read_start_time(pid), read_rss_kib(pid)) else {
            // The compositor process exited between XRes and /proc reads.
            return Ok(None);
        };

        let mut counts = BTreeMap::new();
        let mut resources_total = 0_u64;
        for resource in resources_reply.types {
            let Some(name) = self.atom_name(resource.resource_type)? else {
                return Ok(None);
            };
            if resource.count > 0 {
                resources_total = resources_total.saturating_add(u64::from(resource.count));
                counts.insert(name, resource.count);
            }
        }

        // Reuse the full-allocation pixmap accounting from tests/support/resources.rs: full
        // sizes stay stable when Present changes pixmap reference counts.
        let pixmap_bytes = bytes_reply
            .sizes
            .iter()
            .filter(|value| value.size.spec.type_ == u32::from(AtomEnum::PIXMAP))
            .fold(0_u64, |total, value| {
                total.saturating_add(u64::from(value.size.bytes))
            });

        Ok(Some(CompositorMetrics {
            pid,
            start_time,
            rss_kib,
            pixmap_bytes,
            resources_total,
            pixmap: named_count(&counts, &["PIXMAP"]),
            picture: named_count(&counts, &["PICTURE"]),
            window: named_count(&counts, &["WINDOW"]),
            // Xorg/XLibre register these extension resource types as DamageExt, XFixesRegion,
            // and SyncFence; accept both the short names and the server's registered names.
            damage: named_count(&counts, &["DAMAGE", "DamageExt"]),
            region: named_count(&counts, &["REGION", "XFixesRegion"]),
            gc: named_count(&counts, &["GC"]),
            fence: named_count(&counts, &["FENCE", "SyncFence", "SYNC FENCE"]),
            resources: format_resources(&counts),
        }))
    }

    fn atom_name(&mut self, atom: u32) -> Result<Option<String>> {
        if let Some(cached) = self.atom_names.get(&atom) {
            return Ok(Some(cached.clone()));
        }
        let Some(reply) = non_fatal_x11(self.conn.get_atom_name(atom)?.reply())? else {
            return Ok(None);
        };
        let decoded = String::from_utf8_lossy(&reply.name).into_owned();
        self.atom_names.insert(atom, decoded.clone());
        Ok(Some(decoded))
    }
}

/// Convert a protocol error on a vanished client into `Ok(None)` while keeping connection
/// failures fatal.
fn non_fatal_x11<T>(result: std::result::Result<T, ReplyError>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(ReplyError::X11Error(_)) => Ok(None),
        Err(ReplyError::ConnectionError(error)) => {
            Err(error).context("X11 connection failed during sample")
        }
    }
}

fn named_count(counts: &BTreeMap<String, u32>, names: &[&str]) -> u32 {
    names
        .iter()
        .filter_map(|&name| counts.get(name).copied())
        .fold(0_u32, u32::saturating_add)
}

fn format_resources(counts: &BTreeMap<String, u32>) -> String {
    counts
        .iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join(";")
}

/// Tracks the cumulative count of `Present did not finish a submission` lines across log
/// truncations and rotations.
#[derive(Default)]
struct LogCounter {
    /// Count accumulated from earlier truncated or rotated generations of the log file.
    carried: u64,
    /// Count observed in the current generation of the log file.
    current: u64,
    /// Byte length of the last snapshot observed in the current generation.
    seen_len: usize,
    /// FNV-1a hash of the first `seen_len` bytes of the current generation.
    seen_hash: u64,
    /// Device and inode of the current log file, when read from disk.
    dev_ino: Option<(u64, u64)>,
}

impl LogCounter {
    fn sample_path(&mut self, path: &Path) -> u64 {
        let Ok(metadata) = std::fs::metadata(path) else {
            return self.mark_missing();
        };
        let Ok(bytes) = std::fs::read(path) else {
            return self.mark_missing();
        };
        let file_id = (metadata.dev(), metadata.ino());
        let rotated = self.dev_ino.is_some_and(|previous| previous != file_id);
        self.dev_ino = Some(file_id);
        self.observe_bytes(&bytes, rotated)
    }

    fn mark_missing(&mut self) -> u64 {
        self.carried = self.carried.saturating_add(self.current);
        self.current = 0;
        self.seen_len = 0;
        self.seen_hash = fnv1a(b"");
        self.dev_ino = None;
        self.carried
    }

    fn observe_bytes(&mut self, bytes: &[u8], rotated: bool) -> u64 {
        let same_prefix = !rotated
            && bytes.len() >= self.seen_len
            && bytes
                .get(..self.seen_len)
                .is_some_and(|prefix| fnv1a(prefix) == self.seen_hash);
        if !same_prefix && self.seen_len > 0 {
            self.carried = self.carried.saturating_add(self.current);
        }
        self.current = count_present_timeouts(bytes);
        self.seen_len = bytes.len();
        self.seen_hash = fnv1a(bytes);
        self.carried.saturating_add(self.current)
    }
}

fn count_present_timeouts(bytes: &[u8]) -> u64 {
    let text = String::from_utf8_lossy(bytes);
    u64::try_from(
        text.lines()
            .filter(|line| line.contains(PRESENT_TIMEOUT_WARNING))
            .count(),
    )
    .unwrap_or(u64::MAX)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn read_start_time(pid: u32) -> Result<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .with_context(|| format!("reading /proc/{pid}/stat"))?;
    parse_start_time(&stat)
}

/// Field 22 (`starttime`) of `/proc/PID/stat`, which sits at index 19 after the command
/// name's closing parenthesis.
fn parse_start_time(stat: &str) -> Result<u64> {
    let (_, fields) = stat.rsplit_once(") ").context("invalid process stat")?;
    fields
        .split_whitespace()
        .nth(19)
        .context("missing process start_time field")?
        .parse()
        .context("invalid process start_time")
}

/// The X server's session: the boot's ID and the server's start in clock ticks since boot,
/// which together never repeat, while a start alone can after a reboot.
fn server_session(pid: u32) -> Result<String> {
    let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .context("reading the boot ID")?;
    Ok(format!("{}:{}", boot.trim(), read_start_time(pid)?))
}

fn read_rss_kib(pid: u32) -> Result<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status"))
        .with_context(|| format!("reading /proc/{pid}/status"))?;
    parse_rss_kib(&status)
}

fn parse_rss_kib(status: &str) -> Result<u64> {
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmRSS:"))
        .and_then(|line| line.split_whitespace().next())
        .context("missing process VmRSS")?
        .parse()
        .context("invalid process VmRSS")
}

/// Format a Unix timestamp in seconds as an RFC 3339 UTC string (`YYYY-MM-DDTHH:MM:SSZ`).
fn format_rfc3339_utc(unix_time: u64) -> String {
    let days = unix_time / SECONDS_PER_DAY;
    let day_seconds = unix_time % SECONDS_PER_DAY;
    let hour = day_seconds / 3_600;
    let minute = (day_seconds % 3_600) / 60;
    let second = day_seconds % 60;

    // Howard Hinnant's civil-from-days conversion from the 1970-01-01 Unix epoch.
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn run_sample(args: &SampleArgs) -> Result<()> {
    let mut sampler = SessionSampler::connect(&args.display)?;
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&args.output)
        .with_context(|| format!("opening {}", args.output.display()))?;
    let needs_header = file.metadata()?.len() == 0;
    let mut writer = BufWriter::new(file);
    if needs_header {
        writeln!(writer, "{CSV_HEADER}")?;
        writer.flush()?;
    }

    let mut log_counter = LogCounter::default();
    let interval = Duration::from_secs(args.interval);
    loop {
        let unix_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before the Unix epoch")?
            .as_secs();
        let utc = format_rfc3339_utc(unix_time);
        let idle_ms = sampler.idle_ms()?;
        let compositor = sampler.sample_compositor()?;
        let present_timeouts = args
            .log
            .as_deref()
            .map(|path| log_counter.sample_path(path));
        let server = ServerSample {
            rss_kib: args.server_pid.and_then(|pid| read_rss_kib(pid).ok()),
            session: args.server_pid.and_then(|pid| server_session(pid).ok()),
        };

        write_sample_row(
            &mut writer,
            unix_time,
            &utc,
            compositor.as_ref(),
            idle_ms,
            present_timeouts,
            &server,
        )?;
        writer.flush()?;

        if args.once {
            return Ok(());
        }
        thread::sleep(interval);
    }
}

/// What a row records of the X server, when the sampler was given its PID.
#[derive(Default)]
struct ServerSample {
    rss_kib: Option<u64>,
    /// See `server_session`.
    session: Option<String>,
}

fn write_sample_row(
    writer: &mut impl Write,
    unix_time: u64,
    utc: &str,
    compositor: Option<&CompositorMetrics>,
    idle_ms: u64,
    present_timeouts: Option<u64>,
    server: &ServerSample,
) -> Result<()> {
    let timeouts = present_timeouts.map_or_else(String::new, |count| count.to_string());
    let server_rss = server
        .rss_kib
        .map_or_else(String::new, |rss| rss.to_string());
    let session = server.session.as_deref().unwrap_or_default();
    match compositor {
        Some(metrics) => {
            writeln!(
                writer,
                "{unix_time},{utc},ok,{},{},{},{},{},{},{},{},{},{},{},{},{},{idle_ms},{timeouts},{server_rss},{session}",
                metrics.pid,
                metrics.start_time,
                metrics.rss_kib,
                metrics.pixmap_bytes,
                metrics.resources_total,
                metrics.pixmap,
                metrics.picture,
                metrics.window,
                metrics.damage,
                metrics.region,
                metrics.gc,
                metrics.fence,
                metrics.resources,
            )?;
        }
        None => {
            writeln!(
                writer,
                "{unix_time},{utc},absent,,,,,,,,,,,,,,{idle_ms},{timeouts},{server_rss},{session}"
            )?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Ok,
    Absent,
}

/// One parsed CSV row for `summary` evaluation.
#[derive(Clone, Debug, PartialEq, Eq)]
struct SampleRow {
    unix_time: u64,
    utc: String,
    status: Status,
    pid: Option<u32>,
    start_time: Option<u64>,
    rss_kib: Option<u64>,
    idle_ms: Option<u64>,
    present_timeouts: Option<u64>,
    /// The X server's session, when the sampler was given its PID; see `server_session`.
    session: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SpanEvaluation {
    first_unix: u64,
    first_utc: String,
    last_unix: u64,
    last_utc: String,
    elapsed_seconds: u64,
    passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CrashEvaluation {
    sessions: usize,
    /// Compositor processes beyond the first within a session.
    restarts: usize,
    /// `absent` rows after a session's first `ok` row.
    absent_rows: usize,
    /// `absent` rows before a session's first `ok` row, which do not count.
    startup_absent_rows: usize,
    /// Sessions with no `ok` row at all.
    sessions_without_compositor: usize,
    passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimeoutEvaluation {
    total: u64,
    max_interval_added: u64,
    consecutive_single: bool,
    truncated_intervals: usize,
    passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum MemoryEvaluation {
    Undecidable {
        reason: &'static str,
    },
    Decided {
        first_twice_median_kib: u128,
        first_samples: usize,
        first_used_all: bool,
        last_twice_median_kib: u128,
        last_samples: usize,
        last_used_all: bool,
        passed: bool,
    },
}

impl MemoryEvaluation {
    fn passed(&self) -> bool {
        match self {
            Self::Undecidable { .. } => false,
            Self::Decided { passed, .. } => *passed,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SummaryReport {
    span: SpanEvaluation,
    crashes: CrashEvaluation,
    timeouts: TimeoutEvaluation,
    memory: MemoryEvaluation,
}

impl SummaryReport {
    fn passed(&self) -> bool {
        self.span.passed && self.crashes.passed && self.timeouts.passed && self.memory.passed()
    }
}

fn run_summary(args: &SummaryArgs) -> Result<ExitCode> {
    let file =
        File::open(&args.input).with_context(|| format!("opening {}", args.input.display()))?;
    let rows = parse_csv(BufReader::new(file))?;
    ensure!(!rows.is_empty(), "CSV contains no sample rows");
    let report = evaluate_all(&rows, args.warmup_minutes, args.idle_ms);
    print_report(&report);
    if report.passed() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

fn parse_csv(reader: impl BufRead) -> Result<Vec<SampleRow>> {
    let mut lines = reader
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(text) if text.trim().is_empty() => None,
            Ok(text) => Some(Ok((index + 1, text))),
            Err(error) => Some(Err(error)),
        });
    let Some(header_result) = lines.next() else {
        return Ok(Vec::new());
    };
    let (_, header_line) = header_result.context("reading CSV header")?;
    let columns: BTreeMap<&str, usize> = header_line
        .split(',')
        .enumerate()
        .map(|(index, name)| (name.trim(), index))
        .collect();

    let column = |name: &str| -> Result<usize> {
        columns
            .get(name)
            .copied()
            .with_context(|| format!("missing CSV column '{name}'"))
    };
    let unix_col = column("unix_time")?;
    let utc_col = column("utc")?;
    let status_col = column("status")?;
    let pid_col = column("pid")?;
    let start_col = column("start_time")?;
    let rss_col = column("rss_kib")?;
    let idle_col = column("idle_ms")?;
    let timeouts_col = column("present_timeouts")?;
    let session_col = columns.get("server_session").copied();

    let mut rows = Vec::new();
    for line_result in lines {
        let (line_number, line) = line_result.context("reading CSV line")?;
        let fields: Vec<&str> = line.split(',').collect();
        let field = |index: usize, label: &str| -> Result<&str> {
            fields
                .get(index)
                .copied()
                .map(str::trim)
                .with_context(|| format!("line {line_number}: missing field '{label}'"))
        };
        let unix_time: u64 = field(unix_col, "unix_time")?
            .parse()
            .with_context(|| format!("line {line_number}: invalid unix_time"))?;
        let utc = field(utc_col, "utc")?.to_owned();
        let status = match field(status_col, "status")? {
            "ok" => Status::Ok,
            "absent" => Status::Absent,
            other => bail!("line {line_number}: unknown status '{other}'"),
        };
        let optional_u32 = |index: usize, label: &str| -> Result<Option<u32>> {
            let text = field(index, label)?;
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.parse().with_context(|| {
                    format!("line {line_number}: invalid {label}")
                })?))
            }
        };
        let optional_u64 = |index: usize, label: &str| -> Result<Option<u64>> {
            let text = field(index, label)?;
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.parse().with_context(|| {
                    format!("line {line_number}: invalid {label}")
                })?))
            }
        };
        let pid = optional_u32(pid_col, "pid")?;
        let start_time = optional_u64(start_col, "start_time")?;
        let rss_kib = optional_u64(rss_col, "rss_kib")?;
        let idle_ms = optional_u64(idle_col, "idle_ms")?;
        let present_timeouts = optional_u64(timeouts_col, "present_timeouts")?;
        let session = optional_text(&fields, session_col);
        if status == Status::Ok {
            ensure!(
                pid.is_some() && start_time.is_some() && rss_kib.is_some(),
                "line {line_number}: ok row requires pid, start_time, and rss_kib"
            );
        }
        rows.push(SampleRow {
            unix_time,
            utc,
            status,
            pid,
            start_time,
            rss_kib,
            idle_ms,
            present_timeouts,
            session,
        });
    }
    Ok(rows)
}

/// The text of an optional column, `None` when the column or its value is missing.
fn optional_text(fields: &[&str], index: Option<usize>) -> Option<String> {
    let text = fields.get(index?)?.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn evaluate_all(rows: &[SampleRow], warmup_minutes: u64, min_idle_ms: u64) -> SummaryReport {
    SummaryReport {
        span: evaluate_span(rows),
        crashes: evaluate_crashes(rows),
        timeouts: evaluate_timeouts(rows),
        memory: evaluate_idle_memory(rows, warmup_minutes, min_idle_ms),
    }
}

/// Criterion (a): first and last sample time, and whether at least seven days (604,800 s) are
/// covered.
fn evaluate_span(rows: &[SampleRow]) -> SpanEvaluation {
    let (Some(first), Some(last)) = (rows.first(), rows.last()) else {
        return SpanEvaluation {
            first_unix: 0,
            first_utc: String::new(),
            last_unix: 0,
            last_utc: String::new(),
            elapsed_seconds: 0,
            passed: false,
        };
    };
    let elapsed_seconds = last.unix_time.saturating_sub(first.unix_time);
    SpanEvaluation {
        first_unix: first.unix_time,
        first_utc: first.utc.clone(),
        last_unix: last.unix_time,
        last_utc: last.utc.clone(),
        elapsed_seconds,
        passed: elapsed_seconds >= REQUIRED_DAYS * SECONDS_PER_DAY,
    }
}

/// Criterion (b): within each X server session, one `(pid, start_time)` pair and no `absent`
/// row after the session's first `ok` row. A session ends with its server, as at a logout or a
/// shutdown, and the next one starts a new compositor, which is no crash. `absent` rows before
/// a session's first `ok` row come from a sampler started before the compositor and do not
/// count, but a session in which no compositor ever ran fails. Rows without a session, from a
/// sampler not given the server's PID, form one session.
fn evaluate_crashes(rows: &[SampleRow]) -> CrashEvaluation {
    #[derive(Default)]
    struct Session {
        instances: BTreeSet<(u32, u64)>,
        absent: usize,
        startup_absent: usize,
    }
    let mut sessions: BTreeMap<Option<&str>, Session> = BTreeMap::new();
    for row in rows {
        let session = sessions.entry(row.session.as_deref()).or_default();
        match (row.status, row.pid, row.start_time) {
            (Status::Ok, Some(pid), Some(start_time)) => {
                session.instances.insert((pid, start_time));
            }
            _ if session.instances.is_empty() => {
                session.startup_absent = session.startup_absent.saturating_add(1);
            }
            _ => session.absent = session.absent.saturating_add(1),
        }
    }
    let restarts = sessions
        .values()
        .map(|session| session.instances.len().saturating_sub(1))
        .sum();
    let absent_rows = sessions.values().map(|session| session.absent).sum();
    let startup_absent_rows = sessions
        .values()
        .map(|session| session.startup_absent)
        .sum();
    let sessions_without_compositor = sessions
        .values()
        .filter(|session| session.instances.is_empty())
        .count();
    CrashEvaluation {
        sessions: sessions.len(),
        restarts,
        absent_rows,
        startup_absent_rows,
        sessions_without_compositor,
        passed: restarts == 0 && absent_rows == 0 && sessions_without_compositor == 0,
    }
}

/// Criterion (c): no interval adds two or more Present timeouts, and no two consecutive
/// intervals each add one. If the recorded cumulative count drops because the log was
/// truncated between runs of the sampler, the post-truncation count is treated as the
/// addition in that interval and added to the running total.
fn evaluate_timeouts(rows: &[SampleRow]) -> TimeoutEvaluation {
    let mut total = 0_u64;
    let mut previous_count = 0_u64;
    let mut previous_added = 0_u64;
    let mut max_interval_added = 0_u64;
    let mut consecutive_single = false;
    let mut truncated_intervals = 0_usize;

    for count in rows.iter().filter_map(|row| row.present_timeouts) {
        let added = if count >= previous_count {
            count - previous_count
        } else {
            truncated_intervals = truncated_intervals.saturating_add(1);
            count
        };
        total = total.saturating_add(added);
        max_interval_added = max_interval_added.max(added);
        if previous_added >= 1 && added >= 1 {
            consecutive_single = true;
        }
        previous_count = count;
        previous_added = added;
    }

    TimeoutEvaluation {
        total,
        max_interval_added,
        consecutive_single,
        truncated_intervals,
        passed: max_interval_added < 2 && !consecutive_single,
    }
}

/// Criterion (d): after `--warmup-minutes` of each session, compare the median `rss_kib` of
/// idle samples (`idle_ms >= min_idle_ms`) in the first 24 h after warmup with that of the last
/// 24 h.
/// When a 24 h window has no idle samples, use all `ok` samples in that window and report the
/// fallback. With less than two days of data overall, report the criterion as undecidable.
fn evaluate_idle_memory(
    rows: &[SampleRow],
    warmup_minutes: u64,
    min_idle_ms: u64,
) -> MemoryEvaluation {
    let (Some(first), Some(last)) = (rows.first(), rows.last()) else {
        return MemoryEvaluation::Undecidable {
            reason: "no samples",
        };
    };
    let span_seconds = last.unix_time.saturating_sub(first.unix_time);
    if span_seconds < MIN_MEMORY_DAYS * SECONDS_PER_DAY {
        return MemoryEvaluation::Undecidable {
            reason: "less than two days of data",
        };
    }

    // Each session warms up anew: a logout or a shutdown starts a new compositor.
    let mut session_starts: BTreeMap<Option<&str>, u64> = BTreeMap::new();
    for row in rows {
        session_starts
            .entry(row.session.as_deref())
            .or_insert(row.unix_time);
    }
    let warmup = warmup_minutes.saturating_mul(60);
    let eligible: Vec<&SampleRow> = rows
        .iter()
        .filter(|row| {
            row.status == Status::Ok
                && row.rss_kib.is_some()
                && session_starts
                    .get(&row.session.as_deref())
                    .is_some_and(|&start| row.unix_time >= start.saturating_add(warmup))
        })
        .collect();
    let Some(first_eligible) = eligible.first().map(|row| row.unix_time) else {
        return MemoryEvaluation::Undecidable {
            reason: "no ok samples after warmup",
        };
    };
    let first_window_end = first_eligible.saturating_add(SECONDS_PER_DAY);
    let last_window_start = last.unix_time.saturating_sub(SECONDS_PER_DAY);
    let first_window: Vec<&SampleRow> = eligible
        .iter()
        .copied()
        .filter(|row| row.unix_time <= first_window_end)
        .collect();
    let last_window: Vec<&SampleRow> = eligible
        .iter()
        .copied()
        .filter(|row| row.unix_time >= last_window_start)
        .collect();

    let Some((first_twice_median_kib, first_samples, first_used_all)) =
        window_median_rss(&first_window, min_idle_ms)
    else {
        return MemoryEvaluation::Undecidable {
            reason: "no ok samples in first 24 h after warmup",
        };
    };
    let Some((last_twice_median_kib, last_samples, last_used_all)) =
        window_median_rss(&last_window, min_idle_ms)
    else {
        return MemoryEvaluation::Undecidable {
            reason: "no ok samples in last 24 h",
        };
    };
    if first_twice_median_kib == 0 {
        return MemoryEvaluation::Undecidable {
            reason: "first 24 h median RSS is zero",
        };
    }

    // Pass when the last 24 h median is within 5% of the first 24 h median, either way, as
    // the roadmap's criterion reads; exact integer arithmetic on 2 * median.
    let passed = last_twice_median_kib * 100 <= first_twice_median_kib * 105
        && last_twice_median_kib * 100 >= first_twice_median_kib * 95;
    MemoryEvaluation::Decided {
        first_twice_median_kib,
        first_samples,
        first_used_all,
        last_twice_median_kib,
        last_samples,
        last_used_all,
        passed,
    }
}

/// Select idle samples in `window` (or fall back to all samples in `window` when none are
/// idle) and return `(2 * median_rss_kib, sample_count, used_all_samples_fallback)`.
fn window_median_rss(window: &[&SampleRow], min_idle_ms: u64) -> Option<(u128, usize, bool)> {
    if window.is_empty() {
        return None;
    }
    let mut values: Vec<u64> = window
        .iter()
        .filter(|row| row.idle_ms.unwrap_or(0) >= min_idle_ms)
        .filter_map(|row| row.rss_kib)
        .collect();
    let used_all = values.is_empty();
    if used_all {
        values = window.iter().filter_map(|row| row.rss_kib).collect();
    }
    let twice = twice_median(&mut values)?;
    Some((twice, values.len(), used_all))
}

/// Return `2 * median` of `values` so half-integer medians of even-length slices stay exact.
fn twice_median(values: &mut [u64]) -> Option<u128> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        values.get(mid).map(|&v| u128::from(v) * 2)
    } else {
        let low = values.get(mid.saturating_sub(1)).copied()?;
        let high = values.get(mid).copied()?;
        Some(u128::from(low) + u128::from(high))
    }
}

fn format_half_kib(twice_kib: u128) -> String {
    if twice_kib.is_multiple_of(2) {
        format!("{}", twice_kib / 2)
    } else {
        format!("{}.5", twice_kib / 2)
    }
}

fn print_report(report: &SummaryReport) {
    let whole_days = report.span.elapsed_seconds / SECONDS_PER_DAY;
    let day_hundredths = ((report.span.elapsed_seconds % SECONDS_PER_DAY) * 100) / SECONDS_PER_DAY;
    println!(
        "span: first={} ({}) last={} ({}) days={whole_days}.{day_hundredths:02} (target >= {REQUIRED_DAYS}.00): {}",
        report.span.first_utc,
        report.span.first_unix,
        report.span.last_utc,
        report.span.last_unix,
        status_word(report.span.passed),
    );
    println!(
        "no_crash: sessions={} restarts={} absent_rows={} startup_absent_rows={} sessions_without_compositor={}: {}",
        report.crashes.sessions,
        report.crashes.restarts,
        report.crashes.absent_rows,
        report.crashes.startup_absent_rows,
        report.crashes.sessions_without_compositor,
        status_word(report.crashes.passed),
    );
    println!(
        "present_timeouts: total={} max_interval_added={} consecutive_single={} truncated_intervals={}: {}",
        report.timeouts.total,
        report.timeouts.max_interval_added,
        report.timeouts.consecutive_single,
        report.timeouts.truncated_intervals,
        status_word(report.timeouts.passed),
    );
    match &report.memory {
        MemoryEvaluation::Undecidable { reason } => {
            println!("idle_memory: not yet decidable ({reason}): FAIL");
        }
        MemoryEvaluation::Decided {
            first_twice_median_kib,
            first_samples,
            first_used_all,
            last_twice_median_kib,
            last_samples,
            last_used_all,
            passed,
        } => {
            let first_kind = if *first_used_all {
                "all samples; no idle samples in first 24 h"
            } else {
                "idle samples"
            };
            let last_kind = if *last_used_all {
                "all samples; no idle samples in last 24 h"
            } else {
                "idle samples"
            };
            let signed_bps = if last_twice_median_kib >= first_twice_median_kib {
                let diff = last_twice_median_kib - first_twice_median_kib;
                let bps = (diff * 10_000) / first_twice_median_kib;
                format!("+{}.{:02}%", bps / 100, bps % 100)
            } else {
                let diff = first_twice_median_kib - last_twice_median_kib;
                let bps = (diff * 10_000) / first_twice_median_kib;
                format!("-{}.{:02}%", bps / 100, bps % 100)
            };
            println!(
                "idle_memory: first_24h_median_kib={} (n={first_samples}, {first_kind}) last_24h_median_kib={} (n={last_samples}, {last_kind}) delta={signed_bps} (limit within 5.00% either way): {}",
                format_half_kib(*first_twice_median_kib),
                format_half_kib(*last_twice_median_kib),
                status_word(*passed),
            );
        }
    }
    println!("result: {}", status_word(report.passed()));
}

fn status_word(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn ok_row(
        unix_time: u64,
        pid: u32,
        start_time: u64,
        rss_kib: u64,
        idle_ms: u64,
        present_timeouts: Option<u64>,
    ) -> SampleRow {
        SampleRow {
            unix_time,
            utc: format_rfc3339_utc(unix_time),
            status: Status::Ok,
            pid: Some(pid),
            start_time: Some(start_time),
            rss_kib: Some(rss_kib),
            idle_ms: Some(idle_ms),
            present_timeouts,
            session: None,
        }
    }

    fn in_session(mut row: SampleRow, session: &str) -> SampleRow {
        row.session = Some(session.into());
        row
    }

    fn absent_row(unix_time: u64, idle_ms: u64, present_timeouts: Option<u64>) -> SampleRow {
        SampleRow {
            unix_time,
            utc: format_rfc3339_utc(unix_time),
            status: Status::Absent,
            pid: None,
            start_time: None,
            rss_kib: None,
            idle_ms: Some(idle_ms),
            present_timeouts,
            session: None,
        }
    }

    #[test]
    fn span_requires_seven_full_days() {
        let start = 1_760_000_000_u64;
        let short = [
            ok_row(start, 100, 500, 4000, 600_000, Some(0)),
            ok_row(
                start + REQUIRED_DAYS * SECONDS_PER_DAY - 1,
                100,
                500,
                4000,
                600_000,
                Some(0),
            ),
        ];
        let short_eval = evaluate_span(&short);
        assert!(!short_eval.passed);
        assert_eq!(
            short_eval.elapsed_seconds,
            REQUIRED_DAYS * SECONDS_PER_DAY - 1
        );

        let full = [
            ok_row(start, 100, 500, 4000, 600_000, Some(0)),
            ok_row(
                start + REQUIRED_DAYS * SECONDS_PER_DAY,
                100,
                500,
                4000,
                600_000,
                Some(0),
            ),
        ];
        let full_eval = evaluate_span(&full);
        assert!(full_eval.passed);
        assert_eq!(full_eval.elapsed_seconds, REQUIRED_DAYS * SECONDS_PER_DAY);
    }

    #[test]
    fn crashes_detect_new_pid_same_pid_restart_and_absent_rows() {
        // One continuous process passes.
        let steady = [
            ok_row(1_000, 42, 9_000, 4000, 0, Some(0)),
            ok_row(1_600, 42, 9_000, 4020, 300_000, Some(0)),
        ];
        assert_eq!(
            evaluate_crashes(&steady),
            CrashEvaluation {
                sessions: 1,
                restarts: 0,
                absent_rows: 0,
                startup_absent_rows: 0,
                sessions_without_compositor: 0,
                passed: true,
            }
        );

        // A restart with a new PID fails.
        let new_pid = [
            ok_row(1_000, 42, 9_000, 4000, 0, Some(0)),
            ok_row(1_600, 43, 9_500, 4000, 0, Some(0)),
        ];
        let eval_new_pid = evaluate_crashes(&new_pid);
        assert!(!eval_new_pid.passed);
        assert_eq!(eval_new_pid.restarts, 1);
        assert_eq!(eval_new_pid.absent_rows, 0);

        // A restart that reuses the same PID has a new /proc/PID/stat start_time and fails.
        let same_pid_restart = [
            ok_row(1_000, 42, 9_000, 4000, 0, Some(0)),
            ok_row(1_600, 42, 9_001, 4000, 0, Some(0)),
        ];
        let eval_same_pid = evaluate_crashes(&same_pid_restart);
        assert!(!eval_same_pid.passed);
        assert_eq!(eval_same_pid.restarts, 1);

        // An absent row fails even when the same process returns afterward.
        let with_absence = [
            ok_row(1_000, 42, 9_000, 4000, 0, Some(0)),
            absent_row(1_600, 0, Some(0)),
            ok_row(2_200, 42, 9_000, 4000, 0, Some(0)),
        ];
        let eval_absence = evaluate_crashes(&with_absence);
        assert!(!eval_absence.passed);
        assert_eq!(eval_absence.restarts, 0);
        assert_eq!(eval_absence.absent_rows, 1);
    }

    #[test]
    fn crashes_are_counted_within_server_sessions() {
        // A shutdown between days starts a new server and a new compositor, which passes,
        // and absent rows before a session's compositor starts do not count.
        let days = [
            in_session(absent_row(1_000, 0, Some(0)), "boot-a:100"),
            in_session(ok_row(1_600, 42, 9_000, 4000, 0, Some(0)), "boot-a:100"),
            in_session(ok_row(2_200, 42, 9_000, 4000, 0, Some(0)), "boot-a:100"),
            in_session(absent_row(90_000, 0, Some(0)), "boot-b:100"),
            in_session(ok_row(90_600, 42, 9_000, 4000, 0, Some(0)), "boot-b:100"),
        ];
        assert_eq!(
            evaluate_crashes(&days),
            CrashEvaluation {
                sessions: 2,
                restarts: 0,
                absent_rows: 0,
                startup_absent_rows: 2,
                sessions_without_compositor: 0,
                passed: true,
            }
        );

        // A compositor that restarts while its server keeps running is a crash.
        let mut crashed = days.to_vec();
        crashed.push(in_session(
            ok_row(91_200, 51, 99_000, 4000, 0, Some(0)),
            "boot-b:100",
        ));
        let eval_crashed = evaluate_crashes(&crashed);
        assert!(!eval_crashed.passed);
        assert_eq!(eval_crashed.restarts, 1);

        // So is one that disappears, and a session in which none ever ran fails too.
        let mut vanished = days.to_vec();
        vanished.push(in_session(absent_row(91_200, 0, Some(0)), "boot-b:100"));
        assert_eq!(evaluate_crashes(&vanished).absent_rows, 1);
        let mut empty = days.to_vec();
        empty.push(in_session(absent_row(180_000, 0, Some(0)), "boot-c:100"));
        let eval_empty = evaluate_crashes(&empty);
        assert!(!eval_empty.passed);
        assert_eq!(eval_empty.sessions_without_compositor, 1);
    }

    #[test]
    fn idle_memory_warms_up_in_each_session() {
        // Each day's first hour, where a new compositor starts small, is left out; the idle
        // samples after it decide.
        let t0 = 1_760_000_000_u64;
        let last_day = t0 + REQUIRED_DAYS * SECONDS_PER_DAY;
        let rows = [
            in_session(ok_row(t0, 1, 1, 2_000, 600_000, Some(0)), "a"),
            in_session(ok_row(t0 + 7_200, 1, 1, 4_000, 600_000, Some(0)), "a"),
            in_session(ok_row(last_day - 7_200, 2, 2, 2_000, 600_000, Some(0)), "b"),
            in_session(ok_row(last_day, 2, 2, 4_100, 600_000, Some(0)), "b"),
        ];
        assert_eq!(
            evaluate_idle_memory(&rows, 60, 300_000),
            MemoryEvaluation::Decided {
                first_twice_median_kib: 8_000,
                first_samples: 1,
                first_used_all: false,
                last_twice_median_kib: 8_200,
                last_samples: 1,
                last_used_all: false,
                passed: true,
            }
        );
    }

    #[test]
    fn timeouts_allow_isolated_ones_and_reject_bursts_or_consecutive_intervals() {
        // Isolated single timeouts separated by quiet intervals pass.
        let isolated = [
            ok_row(1_000, 10, 100, 4000, 0, Some(0)),
            ok_row(1_600, 10, 100, 4000, 0, Some(1)),
            ok_row(2_200, 10, 100, 4000, 0, Some(1)),
            ok_row(2_800, 10, 100, 4000, 0, Some(2)),
        ];
        assert_eq!(
            evaluate_timeouts(&isolated),
            TimeoutEvaluation {
                total: 2,
                max_interval_added: 1,
                consecutive_single: false,
                truncated_intervals: 0,
                passed: true,
            }
        );

        // Two timeouts in a single interval fail.
        let burst = [
            ok_row(1_000, 10, 100, 4000, 0, Some(0)),
            ok_row(1_600, 10, 100, 4000, 0, Some(2)),
            ok_row(2_200, 10, 100, 4000, 0, Some(2)),
        ];
        let burst_eval = evaluate_timeouts(&burst);
        assert!(!burst_eval.passed);
        assert_eq!(burst_eval.total, 2);
        assert_eq!(burst_eval.max_interval_added, 2);

        // Two consecutive intervals each adding one timeout fail.
        let consecutive = [
            ok_row(1_000, 10, 100, 4000, 0, Some(0)),
            ok_row(1_600, 10, 100, 4000, 0, Some(1)),
            ok_row(2_200, 10, 100, 4000, 0, Some(2)),
            ok_row(2_800, 10, 100, 4000, 0, Some(2)),
        ];
        let consecutive_eval = evaluate_timeouts(&consecutive);
        assert!(!consecutive_eval.passed);
        assert_eq!(consecutive_eval.total, 2);
        assert!(consecutive_eval.consecutive_single);
    }

    #[test]
    fn truncated_log_counts_remain_cumulative_in_sampler_and_summary() {
        // In the sampler, truncating or rotating the log keeps the cumulative count.
        let mut counter = LogCounter::default();
        let line = format!("WARN {PRESENT_TIMEOUT_WARNING}; replacing its buffers\n");
        assert_eq!(
            counter.observe_bytes(b"INFO compositor started\n", false),
            0
        );
        let first_gen =
            format!("INFO compositor started\n{line}INFO configuration reloaded\n{line}");
        assert_eq!(counter.observe_bytes(first_gen.as_bytes(), false), 2);
        // Truncated to empty.
        assert_eq!(counter.observe_bytes(b"", false), 2);
        // One more warning after truncation brings the cumulative total to 3.
        assert_eq!(counter.observe_bytes(line.as_bytes(), false), 3);
        // Rewritten in place with a new header of at least the same length and one warning.
        let rewritten = format!("INFO restarted with a longer preamble line\n{line}");
        assert_eq!(counter.observe_bytes(rewritten.as_bytes(), false), 4);

        // In summary evaluation, a CSV whose count dropped across a truncation still sums
        // additions and catches consecutive additions across the reset.
        let truncated_isolated = [
            ok_row(1_000, 10, 100, 4000, 0, Some(0)),
            ok_row(1_600, 10, 100, 4000, 0, Some(1)),
            ok_row(2_200, 10, 100, 4000, 0, Some(0)),
            ok_row(2_800, 10, 100, 4000, 0, Some(1)),
        ];
        assert_eq!(
            evaluate_timeouts(&truncated_isolated),
            TimeoutEvaluation {
                total: 2,
                max_interval_added: 1,
                consecutive_single: false,
                truncated_intervals: 1,
                passed: true,
            }
        );

        let truncated_consecutive = [
            ok_row(1_000, 10, 100, 4000, 0, Some(0)),
            ok_row(1_600, 10, 100, 4000, 0, Some(2)),
            ok_row(2_200, 10, 100, 4000, 0, Some(1)),
        ];
        let eval = evaluate_timeouts(&truncated_consecutive);
        assert!(!eval.passed);
        assert_eq!(eval.total, 3);
        assert_eq!(eval.truncated_intervals, 1);
    }

    #[test]
    fn idle_memory_ignores_warmup_checks_five_percent_and_requires_two_days() {
        let t0 = 1_760_000_000_u64;
        // Less than two days of data is not yet decidable and does not pass.
        let one_day = [
            ok_row(t0, 10, 100, 4000, 600_000, Some(0)),
            ok_row(t0 + SECONDS_PER_DAY, 10, 100, 4000, 600_000, Some(0)),
        ];
        let undecidable = evaluate_idle_memory(&one_day, 60, 300_000);
        assert!(!undecidable.passed());
        assert_eq!(
            undecidable,
            MemoryEvaluation::Undecidable {
                reason: "less than two days of data"
            }
        );

        // Seven-day run: high RSS during the first 60-minute warmup and during active samples
        // (idle_ms < 300_000) is ignored; idle median grows from 4000 KiB to 4200 KiB (+5.0%),
        // which passes.
        let warmup_end = t0 + 3_600;
        let last_day = t0 + REQUIRED_DAYS * SECONDS_PER_DAY;
        let mut rows = vec![
            // Warmup sample with artificially low RSS that would fail if not ignored.
            ok_row(t0 + 600, 10, 100, 2000, 600_000, Some(0)),
            ok_row(t0 + 1_200, 10, 100, 2000, 600_000, Some(0)),
            ok_row(t0 + 1_800, 10, 100, 2000, 600_000, Some(0)),
            // First 24 h after warmup: three idle samples (median 4000) and one active spike.
            ok_row(warmup_end + 600, 10, 100, 3980, 300_000, Some(0)),
            ok_row(warmup_end + 1_200, 10, 100, 4000, 450_000, Some(0)),
            ok_row(warmup_end + 1_800, 10, 100, 4020, 500_000, Some(0)),
            ok_row(warmup_end + 2_400, 10, 100, 9000, 10_000, Some(0)),
            // Last 24 h: three idle samples with median 4200 (+5.00% exactly).
            ok_row(last_day - 3_600, 10, 100, 4190, 350_000, Some(0)),
            ok_row(last_day - 1_800, 10, 100, 4200, 400_000, Some(0)),
            ok_row(last_day, 10, 100, 4210, 600_000, Some(0)),
        ];
        assert_eq!(
            evaluate_idle_memory(&rows, 60, 300_000),
            MemoryEvaluation::Decided {
                first_twice_median_kib: 8_000,
                first_samples: 3,
                first_used_all: false,
                last_twice_median_kib: 8_400,
                last_samples: 3,
                last_used_all: false,
                passed: true,
            }
        );

        // Raising the last-day idle median to 4204 KiB (+5.10%) fails.
        for row in rows.iter_mut().rev().take(3) {
            row.rss_kib = Some(4204);
        }
        let failed = evaluate_idle_memory(&rows, 60, 300_000);
        assert!(!failed.passed());

        // So does lowering it below 3800 KiB (-5%), and 3800 KiB itself passes.
        for (rss, passes) in [(3_800, true), (3_796, false)] {
            for row in rows.iter_mut().rev().take(3) {
                row.rss_kib = Some(rss);
            }
            assert_eq!(evaluate_idle_memory(&rows, 60, 300_000).passed(), passes);
        }
    }

    #[test]
    fn idle_memory_falls_back_to_all_window_samples_when_none_are_idle() {
        let t0 = 1_760_000_000_u64;
        let warmup_end = t0 + 3_600;
        let last_day = t0 + REQUIRED_DAYS * SECONDS_PER_DAY;
        // First 24 h has only active samples (idle_ms < 300_000); last 24 h has idle samples.
        let rows = [
            ok_row(t0, 10, 100, 3900, 0, Some(0)),
            ok_row(warmup_end + 600, 10, 100, 4000, 5_000, Some(0)),
            ok_row(warmup_end + 1_200, 10, 100, 4100, 15_000, Some(0)),
            ok_row(last_day - 600, 10, 100, 4080, 400_000, Some(0)),
            ok_row(last_day, 10, 100, 4120, 500_000, Some(0)),
        ];
        assert_eq!(
            evaluate_idle_memory(&rows, 60, 300_000),
            MemoryEvaluation::Decided {
                first_twice_median_kib: 8_100,
                first_samples: 2,
                first_used_all: true,
                last_twice_median_kib: 8_200,
                last_samples: 2,
                last_used_all: false,
                passed: true,
            }
        );
    }

    #[test]
    fn csv_round_trip_and_proc_parsers_handle_ok_and_absent_rows() {
        let mut buffer = Vec::new();
        writeln!(buffer, "{CSV_HEADER}").unwrap();
        let metrics = CompositorMetrics {
            pid: 321,
            start_time: 123_456,
            rss_kib: 4_096,
            pixmap_bytes: 614_404,
            resources_total: 12,
            pixmap: 2,
            picture: 4,
            window: 1,
            damage: 2,
            region: 1,
            gc: 1,
            fence: 1,
            resources: "DamageExt=2;GC=1;PICTURE=4;PIXMAP=2;SyncFence=1;WINDOW=1;XFixesRegion=1"
                .into(),
        };
        write_sample_row(
            &mut buffer,
            1_793_833_365,
            "2026-11-04T23:02:45Z",
            Some(&metrics),
            350_000,
            Some(0),
            &ServerSample {
                rss_kib: Some(28_000),
                session: Some("boot:4213705".into()),
            },
        )
        .unwrap();
        write_sample_row(
            &mut buffer,
            1_793_833_965,
            "2026-11-04T23:12:45Z",
            None,
            1_200,
            Some(1),
            &ServerSample::default(),
        )
        .unwrap();

        let parsed = parse_csv(Cursor::new(buffer)).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed.first(),
            Some(&SampleRow {
                unix_time: 1_793_833_365,
                utc: "2026-11-04T23:02:45Z".into(),
                status: Status::Ok,
                pid: Some(321),
                start_time: Some(123_456),
                rss_kib: Some(4_096),
                idle_ms: Some(350_000),
                present_timeouts: Some(0),
                session: Some("boot:4213705".into()),
            })
        );
        assert_eq!(
            parsed.get(1),
            Some(&SampleRow {
                unix_time: 1_793_833_965,
                utc: "2026-11-04T23:12:45Z".into(),
                status: Status::Absent,
                pid: None,
                start_time: None,
                rss_kib: None,
                idle_ms: Some(1_200),
                present_timeouts: Some(1),
                session: None,
            })
        );

        // /proc/PID/stat with spaces and parentheses inside comm still extracts field 22.
        let stat = "321 (compust (test)) S 1 321 321 0 -1 4194304 100 0 0 0 10 5 0 0 20 0 1 0 987654 12345678 1024";
        assert_eq!(parse_start_time(stat).unwrap(), 987_654);
        let status = "Name:\tcompust\nVmSize:\t   16384 kB\nVmRSS:\t    4096 kB\n";
        assert_eq!(parse_rss_kib(status).unwrap(), 4_096);
        assert_eq!(format_rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339_utc(1_793_833_361), "2026-11-04T23:02:41Z");
    }
}
