use super::{content, everyday, graphics, productivity, stats, storage};
use crate::models::{BenchmarkProfile, BenchmarkProgress, SampleStats, WorkloadResult};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

type WorkloadFn = fn(&BenchmarkProfile, &AtomicBool) -> Result<(f64, String), String>;

pub struct WorkloadSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub suite: &'static str,
    pub unit: &'static str,
    pub higher_is_better: bool,
    pub weight: f64,
    pub run: WorkloadFn,
}

pub fn all_workloads() -> Vec<WorkloadSpec> {
    vec![
        WorkloadSpec {
            key: "json_parse",
            label: "JSON / data parsing",
            suite: "everyday",
            unit: "MB/s",
            higher_is_better: true,
            weight: 0.25,
            run: everyday::json_parse,
        },
        WorkloadSpec {
            key: "text_search",
            label: "Text search / indexing",
            suite: "everyday",
            unit: "MB/s",
            higher_is_better: true,
            weight: 0.25,
            run: everyday::text_search,
        },
        WorkloadSpec {
            key: "compression",
            label: "Compression",
            suite: "everyday",
            unit: "MB/s",
            higher_is_better: true,
            weight: 0.25,
            run: everyday::compression,
        },
        WorkloadSpec {
            key: "small_files",
            label: "Small-file operations",
            suite: "everyday",
            unit: "ops/s",
            higher_is_better: true,
            weight: 0.25,
            run: everyday::small_files,
        },
        WorkloadSpec {
            key: "spreadsheet",
            label: "Spreadsheet formulas",
            suite: "productivity",
            unit: "Mcells/s",
            higher_is_better: true,
            weight: 0.30,
            run: productivity::spreadsheet,
        },
        WorkloadSpec {
            key: "sort_aggregate",
            label: "Sort / aggregation",
            suite: "productivity",
            unit: "Mrows/s",
            higher_is_better: true,
            weight: 0.25,
            run: productivity::sort_aggregate,
        },
        WorkloadSpec {
            key: "document_transform",
            label: "Document transforms",
            suite: "productivity",
            unit: "MB/s",
            higher_is_better: true,
            weight: 0.25,
            run: productivity::document_transform,
        },
        WorkloadSpec {
            key: "archive",
            label: "Archive creation",
            suite: "productivity",
            unit: "MB/s",
            higher_is_better: true,
            weight: 0.20,
            run: productivity::archive,
        },
        WorkloadSpec {
            key: "image_pipeline",
            label: "Image edit pipeline",
            suite: "content",
            unit: "MP/s",
            higher_is_better: true,
            weight: 0.45,
            run: content::image_pipeline,
        },
        WorkloadSpec {
            key: "cpu_render",
            label: "CPU scene render",
            suite: "content",
            unit: "MP/s",
            higher_is_better: true,
            weight: 0.55,
            run: content::cpu_render,
        },
        WorkloadSpec {
            key: "gpu_compute",
            label: "GPU compute",
            suite: "graphics",
            unit: "GFLOPS",
            higher_is_better: true,
            weight: 0.55,
            run: graphics::gpu_compute,
        },
        WorkloadSpec {
            key: "gpu_offscreen",
            label: "GPU offscreen render",
            suite: "graphics",
            unit: "FPS",
            higher_is_better: true,
            weight: 0.35,
            run: graphics::gpu_offscreen,
        },
        WorkloadSpec {
            key: "gpu_transfer",
            label: "GPU transfer bandwidth",
            suite: "graphics",
            unit: "GB/s",
            higher_is_better: true,
            weight: 0.10,
            run: graphics::gpu_transfer,
        },
        WorkloadSpec {
            key: "storage_sequential",
            label: "Sequential storage",
            suite: "storage",
            unit: "MB/s",
            higher_is_better: true,
            weight: 0.55,
            run: storage::sequential,
        },
        WorkloadSpec {
            key: "storage_random",
            label: "Random storage",
            suite: "storage",
            unit: "IOPS",
            higher_is_better: true,
            weight: 0.45,
            run: storage::random,
        },
    ]
}

#[allow(clippy::too_many_arguments)]
pub fn run_workload(
    app: &AppHandle,
    started: Instant,
    profile: BenchmarkProfile,
    cancelled: &AtomicBool,
    spec: &WorkloadSpec,
    suite_index: usize,
    suite_count: usize,
    workload_index: usize,
    workload_count: usize,
) -> WorkloadResult {
    let warmups = profile.warmups();
    let samples_needed = profile.samples();
    let mut measured = Vec::new();
    let mut hash = None;
    let mut last_error = None;

    for warmup in 0..warmups {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        emit_progress(
            app,
            started,
            profile,
            spec,
            suite_index,
            suite_count,
            workload_index,
            workload_count,
            &format!("Warming up {} ({}/{})", spec.label, warmup + 1, warmups),
        );
        let _ = (spec.run)(&profile, cancelled);
    }

    for sample_index in 0..samples_needed {
        if cancelled.load(Ordering::Relaxed) {
            break;
        }
        emit_progress(
            app,
            started,
            profile,
            spec,
            suite_index,
            suite_count,
            workload_index,
            workload_count,
            &format!(
                "Measuring {} ({}/{})",
                spec.label,
                sample_index + 1,
                samples_needed
            ),
        );
        match (spec.run)(&profile, cancelled) {
            Ok((value, output_hash)) => {
                measured.push(value);
                hash = Some(output_hash);
            }
            Err(error) => {
                last_error = Some(error);
                break;
            }
        }
    }

    let sample_stats = stats::summarize(&measured);
    let reliable = sample_stats
        .as_ref()
        .map(stats::is_reliable)
        .unwrap_or(false);
    let valid = sample_stats.is_some() && reliable && !cancelled.load(Ordering::Relaxed);

    WorkloadResult {
        key: spec.key.into(),
        label: spec.label.into(),
        suite: spec.suite.into(),
        unit: spec.unit.into(),
        higher_is_better: spec.higher_is_better,
        stats: sample_stats.clone(),
        score: None,
        grade: None,
        weight: spec.weight,
        valid,
        reliability: sample_stats
            .as_ref()
            .map(|s| s.reliability.clone())
            .unwrap_or_else(|| "unavailable".into()),
        explanation: explanation(spec, &sample_stats, last_error.as_deref(), valid),
        output_hash: hash,
    }
}

fn explanation(
    spec: &WorkloadSpec,
    sample_stats: &Option<SampleStats>,
    error: Option<&str>,
    valid: bool,
) -> String {
    if let Some(error) = error {
        return format!("{} failed: {error}", spec.label);
    }
    match sample_stats {
        Some(stats) if valid => format!(
            "Median {:.2} {} across {} samples (CV {:.1}%, {}).",
            stats.median,
            spec.unit,
            stats.samples.len(),
            stats.coefficient_of_variation,
            stats.reliability
        ),
        Some(stats) => format!(
            "Measured {:.2} {} but reliability was {} (CV {:.1}%).",
            stats.median, spec.unit, stats.reliability, stats.coefficient_of_variation
        ),
        None => format!("{} did not produce a complete result.", spec.label),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_progress(
    app: &AppHandle,
    started: Instant,
    profile: BenchmarkProfile,
    spec: &WorkloadSpec,
    suite_index: usize,
    suite_count: usize,
    workload_index: usize,
    workload_count: usize,
    message: &str,
) {
    let total_steps = (suite_count * workload_count).max(1) as f64;
    let completed = (suite_index * workload_count + workload_index) as f64;
    let percent = ((completed / total_steps) * 90.0 + 5.0).clamp(1.0, 95.0) as u8;
    let elapsed = started.elapsed();
    let estimated_remaining = if completed > 0.0 {
        let per = elapsed.as_millis() as f64 / completed;
        Some(((total_steps - completed) * per) as u128)
    } else {
        Some((profile.estimated_seconds() as u128) * 1000)
    };
    let _ = app.emit(
        "benchmark-progress",
        BenchmarkProgress {
            stage: format!("{}-{}", spec.suite, spec.key),
            suite: spec.suite.into(),
            workload: spec.label.into(),
            percent,
            message: message.into(),
            elapsed_ms: elapsed.as_millis(),
            estimated_remaining_ms: estimated_remaining,
        },
    );
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

pub fn timed_throughput<F>(cancelled: &AtomicBool, bytes_or_units: f64, mut work: F) -> Option<f64>
where
    F: FnMut() -> bool,
{
    if cancelled.load(Ordering::Relaxed) {
        return None;
    }
    let started = Instant::now();
    if !work() {
        return None;
    }
    let seconds = started
        .elapsed()
        .as_secs_f64()
        .max(Duration::from_micros(1).as_secs_f64());
    Some(bytes_or_units / seconds)
}
