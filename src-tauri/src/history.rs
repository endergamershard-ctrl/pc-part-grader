use crate::models::{BenchmarkReport, HistorySummary};
use crate::scoring;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
static TEST_HISTORY_ROOT: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

fn history_root() -> Result<PathBuf, String> {
    #[cfg(test)]
    if let Some(root) = TEST_HISTORY_ROOT.lock().unwrap().clone() {
        fs::create_dir_all(&root)
            .map_err(|e| format!("Could not create history directory: {e}"))?;
        return Ok(root);
    }
    let base =
        dirs::data_dir().ok_or_else(|| "Could not resolve app data directory".to_string())?;
    let root = base.join("pc-part-grader").join("history");
    fs::create_dir_all(&root).map_err(|e| format!("Could not create history directory: {e}"))?;
    Ok(root)
}

fn report_path(root: &Path, id: &str) -> PathBuf {
    root.join(format!("{id}.json"))
}

pub fn save_report(report: &BenchmarkReport) -> Result<(), String> {
    let root = history_root()?;
    let path = report_path(&root, &report.id);
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(report).map_err(|e| e.to_string())?;
    fs::write(&tmp, json).map_err(|e| format!("Could not write history temp file: {e}"))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Could not finalize history file: {e}"))?;
    Ok(())
}

pub fn list_reports() -> Result<Vec<HistorySummary>, String> {
    let root = history_root()?;
    let mut items = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(report) = load_path(&path) {
            items.push(HistorySummary {
                id: report.id,
                profile: report.profile,
                generated_at_unix_ms: report.generated_at_unix_ms,
                total_score: report.total_score,
                letter_grade: report.letter_grade,
                cpu_name: report.hardware.cpu.name,
                gpu_name: report
                    .hardware
                    .gpus
                    .first()
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "Unknown GPU".into()),
                cancelled: report.cancelled,
            });
        }
    }
    items.sort_by_key(|item| std::cmp::Reverse(item.generated_at_unix_ms));
    Ok(items)
}

pub fn load_report(id: &str) -> Result<BenchmarkReport, String> {
    let root = history_root()?;
    load_path(&report_path(&root, id))
}

pub fn delete_report(id: &str) -> Result<(), String> {
    let root = history_root()?;
    let path = report_path(&root, id);
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("Could not delete report: {e}"))?;
    }
    Ok(())
}

pub fn enrich_with_history(report: &mut BenchmarkReport) -> Result<(), String> {
    let history = list_reports()?;
    let previous = history
        .iter()
        .find(|item| item.id != report.id)
        .and_then(|item| item.total_score);
    let best = history
        .iter()
        .filter(|item| item.id != report.id)
        .filter_map(|item| item.total_score)
        .fold(None, |acc, score| {
            Some(acc.map_or(score, |best: f64| best.max(score)))
        });
    scoring::attach_history_comparison(report, previous, best);
    Ok(())
}

fn load_path(path: &Path) -> Result<BenchmarkReport, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| format!("Invalid history file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    fn sample_report(id: &str, score: f64) -> BenchmarkReport {
        BenchmarkReport {
            id: id.into(),
            scoring_version: "2.0.0".into(),
            profile: BenchmarkProfile::Standard,
            generated_at_unix_ms: 1,
            duration_ms: 1000,
            hardware: HardwareInfo {
                os: "Test".into(),
                hostname: "host".into(),
                cpu: CpuInfo {
                    name: "CPU".into(),
                    physical_cores: 4,
                    logical_cores: 8,
                    frequency_mhz: 3000,
                },
                memory: MemoryInfo {
                    total_bytes: 8 << 30,
                    available_bytes: 4 << 30,
                },
                gpus: vec![],
                disks: vec![],
            },
            environment: RunEnvironment {
                os: "Test".into(),
                cpu_usage_percent: 1.0,
                available_memory_bytes: 4 << 30,
                total_memory_bytes: 8 << 30,
                thermal_celsius: None,
                warnings: vec![],
            },
            suites: vec![],
            components: vec![],
            total_score: Some(score),
            total_grade: Some(70.0),
            letter_grade: "B".into(),
            comparison: LocalComparison {
                previous_total: None,
                best_total: None,
                delta_vs_previous: None,
                delta_vs_best: None,
                reference_tier: None,
                reference_note: String::new(),
            },
            notes: vec![],
            cancelled: false,
        }
    }

    #[test]
    fn save_list_and_compare_history() {
        let _guard = LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        *TEST_HISTORY_ROOT.lock().unwrap() = Some(temp.path().to_path_buf());
        let first = sample_report("hist-a", 1000.0);
        save_report(&first).unwrap();
        let mut second = sample_report("hist-b", 1100.0);
        enrich_with_history(&mut second).unwrap();
        save_report(&second).unwrap();
        assert_eq!(second.comparison.previous_total, Some(1000.0));
        assert_eq!(second.comparison.delta_vs_previous, Some(100.0));
        *TEST_HISTORY_ROOT.lock().unwrap() = None;
    }
}
