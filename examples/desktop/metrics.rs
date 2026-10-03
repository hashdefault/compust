use anyhow::{Context, Result, ensure};
use std::{io::Write, process::Command, str::FromStr, time::Duration};

#[derive(Clone)]
pub(super) struct Process {
    label: String,
    pid: u32,
}

impl FromStr for Process {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let (label, pid) = value.split_once(':').context("expected label:pid")?;
        ensure!(
            !label.is_empty()
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_'),
            "invalid process label"
        );
        let pid = pid.parse()?;
        ensure!(pid > 0, "process ID must be positive");
        Ok(Self {
            label: label.into(),
            pid,
        })
    }
}

impl Process {
    pub(super) fn probe() -> Self {
        Self {
            label: "probe".into(),
            pid: std::process::id(),
        }
    }
}

struct Usage {
    process: Process,
    start_time: u64,
    ticks: u64,
    rss: u64,
}

pub(super) struct Snapshot(Vec<Usage>);

impl Snapshot {
    pub(super) fn read(processes: &[Process]) -> Result<Self> {
        let mut values = Vec::with_capacity(processes.len());
        for process in processes {
            let stat = std::fs::read_to_string(format!("/proc/{}/stat", process.pid))?;
            let (_, fields) = stat.rsplit_once(") ").context("invalid process stat")?;
            let fields: Vec<&str> = fields.split_whitespace().collect();
            let field = |index: usize| -> Result<u64> {
                fields
                    .get(index)
                    .context("missing process stat field")?
                    .parse()
                    .context("invalid process stat value")
            };
            let status = std::fs::read_to_string(format!("/proc/{}/status", process.pid))?;
            let rss = status
                .lines()
                .find_map(|line| line.strip_prefix("VmRSS:"))
                .and_then(|line| line.split_whitespace().next())
                .context("missing process RSS")?
                .parse()?;
            values.push(Usage {
                process: process.clone(),
                start_time: field(19)?,
                ticks: field(11)?
                    .checked_add(field(12)?)
                    .context("CPU ticks overflow")?,
                rss,
            });
        }
        Ok(Self(values))
    }

    pub(super) fn write_delta(
        &self,
        after: &Self,
        phase: &str,
        elapsed: Duration,
        output: &mut impl Write,
    ) -> Result<()> {
        let clock = Command::new("getconf").arg("CLK_TCK").output()?;
        ensure!(clock.status.success(), "getconf CLK_TCK failed");
        let frequency: u32 = std::str::from_utf8(&clock.stdout)?.trim().parse()?;
        ensure!(frequency > 0, "invalid CPU clock frequency");
        ensure!(self.0.len() == after.0.len(), "different process snapshots");
        for (before, after) in self.0.iter().zip(&after.0) {
            ensure!(
                before.process.pid == after.process.pid && before.start_time == after.start_time,
                "process changed during measurement"
            );
            let ticks = u32::try_from(
                after
                    .ticks
                    .checked_sub(before.ticks)
                    .context("CPU ticks decreased")?,
            )?;
            let percent = f64::from(ticks) / f64::from(frequency) / elapsed.as_secs_f64() * 100.0;
            writeln!(
                output,
                "{phase},{},{:.6},{ticks},{frequency},{percent:.4},{},{}",
                before.process.label,
                elapsed.as_secs_f64(),
                before.rss,
                after.rss
            )?;
        }
        Ok(())
    }
}
