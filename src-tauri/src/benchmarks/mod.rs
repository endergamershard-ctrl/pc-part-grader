pub mod content;
pub mod everyday;
pub mod graphics;
pub mod productivity;
pub mod runner;
pub mod stats;
pub mod storage;

use crate::hardware;
use crate::models::{
    BenchmarkProfile, BenchmarkProgress, BenchmarkReport, RunEnvironment, SuiteScore,
};
use crate::scoring;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct BenchmarkControl {
    pub cancelled: AtomicBool,
    pub running: AtomicBool,
}

impl BenchmarkControl {
    pub fn begin(&self) -> Result<(), String> {
        self.running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "A benchmark is already running.".to_string())?;
        self.cancelled.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub fn finish(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

pub fn execute(
    app: &AppHandle,
    control: &BenchmarkControl,
    profile: BenchmarkProfile,
) -> BenchmarkReport {
    let started = Instant::now();
    let _ = app.emit(
        "benchmark-progress",
        BenchmarkProgress {
            stage: "detecting".into(),
            suite: "system".into(),
            workload: "Hardware detection".into(),
            percent: 2,
            message: "Reading installed hardware".into(),
            elapsed_ms: 0,
            estimated_remaining_ms: Some((profile.estimated_seconds() as u128) * 1000),
        },
    );

    let hardware = hardware::detect();
    let environment = capture_environment(&hardware);
    let mut notes = environment.warnings.clone();
    let workloads = runner::all_workloads();
    let suite_order = ["everyday", "productivity", "content", "graphics", "storage"];
    let mut suite_map: BTreeMap<&str, Vec<_>> = BTreeMap::new();
    for workload in &workloads {
        suite_map.entry(workload.suite).or_default().push(workload);
    }

    let mut suite_scores = Vec::new();
    for (suite_index, suite_key) in suite_order.iter().enumerate() {
        if control.is_cancelled() {
            break;
        }
        let Some(suite_workloads) = suite_map.get(suite_key) else {
            continue;
        };
        let mut results = Vec::new();
        for (workload_index, spec) in suite_workloads.iter().enumerate() {
            if control.is_cancelled() {
                break;
            }
            results.push(runner::run_workload(
                app,
                started,
                profile,
                &control.cancelled,
                spec,
                suite_index,
                suite_order.len(),
                workload_index,
                suite_workloads.len(),
            ));
        }
        suite_scores.push(SuiteScore {
            key: (*suite_key).into(),
            label: suite_label(suite_key).into(),
            score: None,
            grade: None,
            weight: suite_weight(suite_key),
            reliability: "pending".into(),
            bottleneck: None,
            workloads: results,
        });
    }

    if control.is_cancelled() {
        notes.push("Benchmark cancelled; this report contains partial results.".into());
    }

    let _ = app.emit(
        "benchmark-progress",
        BenchmarkProgress {
            stage: "scoring".into(),
            suite: "system".into(),
            workload: "Scoring".into(),
            percent: 96,
            message: "Calculating suite and overall scores".into(),
            elapsed_ms: started.elapsed().as_millis(),
            estimated_remaining_ms: Some(1_000),
        },
    );

    let mut report = scoring::build_report(
        Uuid::new_v4().to_string(),
        profile,
        hardware,
        environment,
        suite_scores,
        started.elapsed().as_millis(),
        control.is_cancelled(),
        notes,
    );

    let _ = app.emit(
        "benchmark-progress",
        BenchmarkProgress {
            stage: "complete".into(),
            suite: "system".into(),
            workload: "Complete".into(),
            percent: 100,
            message: "Benchmark complete".into(),
            elapsed_ms: started.elapsed().as_millis(),
            estimated_remaining_ms: Some(0),
        },
    );

    // Attach empty comparison for now; history layer fills this in.
    report.comparison.reference_note =
        "Local reference tiers available after scoring model 2.0 baselines load.".into();
    report
}

fn suite_label(key: &str) -> &'static str {
    match key {
        "everyday" => "Everyday",
        "productivity" => "Productivity",
        "content" => "Content Creation",
        "graphics" => "Graphics",
        "storage" => "Storage",
        _ => "Suite",
    }
}

fn suite_weight(key: &str) -> f64 {
    match key {
        "everyday" => 0.20,
        "productivity" => 0.20,
        "content" => 0.20,
        "graphics" => 0.20,
        "storage" => 0.20,
        _ => 0.20,
    }
}

fn capture_environment(hardware: &crate::models::HardwareInfo) -> RunEnvironment {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_usage();
    std::thread::sleep(std::time::Duration::from_millis(120));
    system.refresh_cpu_usage();
    system.refresh_memory();
    let cpu_usage = system.global_cpu_usage();
    let mut warnings = Vec::new();
    if cpu_usage > 35.0 {
        warnings.push(format!(
            "Background CPU load is high ({cpu_usage:.0}%). Results may be less reliable."
        ));
    }
    if hardware.memory.available_bytes < 2 * 1024 * 1024 * 1024 {
        warnings
            .push("Less than 2 GiB memory available; close other apps for better results.".into());
    }
    let components = sysinfo::Components::new_with_refreshed_list();
    let thermal = components.iter().find_map(|component| {
        let label = component.label().to_ascii_lowercase();
        if label.contains("cpu") || label.contains("tdie") || label.contains("package") {
            component.temperature()
        } else {
            None
        }
    });
    if let Some(temp) = thermal {
        if temp > 90.0 {
            warnings.push(format!(
                "CPU temperature is elevated ({temp:.0}°C). Thermal throttling may reduce scores."
            ));
        }
    }
    RunEnvironment {
        os: hardware.os.clone(),
        cpu_usage_percent: cpu_usage,
        available_memory_bytes: hardware.memory.available_bytes,
        total_memory_bytes: hardware.memory.total_bytes,
        thermal_celsius: thermal,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::BenchmarkControl;

    #[test]
    fn control_prevents_parallel_runs_and_can_cancel() {
        let control = BenchmarkControl::default();
        assert!(control.begin().is_ok());
        assert!(control.begin().is_err());
        control.cancel();
        assert!(control.is_cancelled());
        control.finish();
        assert!(control.begin().is_ok());
    }
}
