mod benchmarks;
mod community;
mod hardware;
mod history;
mod models;
mod scoring;

use benchmarks::BenchmarkControl;
use models::{
    BenchmarkProfile, BenchmarkReport, CommunitySettings, HardwareInfo, HistorySummary,
    RedactedCommunityPayload,
};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
fn get_hardware() -> HardwareInfo {
    hardware::detect()
}

#[tauri::command]
fn cancel_benchmark(control: State<'_, Arc<BenchmarkControl>>) {
    control.cancel();
}

#[tauri::command]
async fn run_benchmark(
    app: AppHandle,
    control: State<'_, Arc<BenchmarkControl>>,
    profile: String,
) -> Result<BenchmarkReport, String> {
    let profile = match profile.as_str() {
        "extended" => BenchmarkProfile::Extended,
        _ => BenchmarkProfile::Standard,
    };
    control.begin()?;
    let control = Arc::clone(control.inner());
    tauri::async_runtime::spawn_blocking(move || {
        let mut report = benchmarks::execute(&app, &control, profile);
        let _ = history::enrich_with_history(&mut report);
        if let Err(error) = history::save_report(&report) {
            report
                .notes
                .push(format!("Could not save history: {error}"));
        }
        control.finish();
        report
    })
    .await
    .map_err(|error| format!("Benchmark worker failed: {error}"))
}

#[tauri::command]
fn list_history() -> Result<Vec<HistorySummary>, String> {
    history::list_reports()
}

#[tauri::command]
fn load_history(id: String) -> Result<BenchmarkReport, String> {
    history::load_report(&id)
}

#[tauri::command]
fn delete_history(id: String) -> Result<(), String> {
    history::delete_report(&id)
}

#[tauri::command]
fn get_community_settings() -> CommunitySettings {
    community::load_settings()
}

#[tauri::command]
fn set_community_settings(settings: CommunitySettings) -> Result<CommunitySettings, String> {
    community::save_settings(settings)
}

#[tauri::command]
fn preview_community_payload(id: String) -> Result<RedactedCommunityPayload, String> {
    let report = history::load_report(&id)?;
    community::preview_upload(&report)
}

#[tauri::command]
fn export_report_json(id: String) -> Result<String, String> {
    let mut report = history::load_report(&id)?;
    report.hardware.hostname = "redacted".into();
    serde_json::to_string_pretty(&report).map_err(|e| e.to_string())
}

/// Developer calibration entrypoint used by `src-tauri/src/bin/calibrate.rs`.
pub fn calibrate_main() -> Result<(), String> {
    use benchmarks::runner;
    use models::BenchmarkProfile;
    use std::sync::atomic::AtomicBool;

    let profile = match std::env::args().nth(1).as_deref() {
        Some("extended") => BenchmarkProfile::Extended,
        _ => BenchmarkProfile::Standard,
    };
    let cancelled = AtomicBool::new(false);
    let mut samples = serde_json::Map::new();
    for spec in runner::all_workloads() {
        println!("calibrating {}...", spec.key);
        let mut values = Vec::new();
        for _ in 0..profile.samples() {
            match (spec.run)(&profile, &cancelled) {
                Ok((value, _)) => values.push(value),
                Err(error) => {
                    eprintln!("  skipped {}: {error}", spec.key);
                    break;
                }
            }
        }
        if let Some(stats) = benchmarks::stats::summarize(&values) {
            samples.insert(
                spec.key.to_string(),
                serde_json::json!({
                    "median": stats.median,
                    "cv": stats.coefficient_of_variation,
                    "samples": stats.samples,
                }),
            );
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "scoringVersion": scoring::SCORING_VERSION,
            "profile": profile.as_str(),
            "workloads": samples,
        }))
        .map_err(|e| e.to_string())?
    );
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(BenchmarkControl::default()))
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_hardware,
            run_benchmark,
            cancel_benchmark,
            list_history,
            load_history,
            delete_history,
            get_community_settings,
            set_community_settings,
            preview_community_payload,
            export_report_json
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
