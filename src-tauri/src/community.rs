use crate::models::{BenchmarkReport, CommunitySettings, RedactedCommunityPayload};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

fn settings_path() -> Result<PathBuf, String> {
    let base =
        dirs::config_dir().ok_or_else(|| "Could not resolve config directory".to_string())?;
    let dir = base.join("pc-part-grader");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("community.json"))
}

pub fn load_settings() -> CommunitySettings {
    let Ok(path) = settings_path() else {
        return CommunitySettings::default();
    };
    fs::read_to_string(path)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: CommunitySettings) -> Result<CommunitySettings, String> {
    let mut settings = settings;
    // Disabled by default until a future service is configured and opted into.
    settings.enabled = settings.opt_in
        && settings
            .endpoint
            .as_ref()
            .map(|endpoint| !endpoint.trim().is_empty())
            .unwrap_or(false);
    let path = settings_path()?;
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(settings)
}

pub fn redact_report(report: &BenchmarkReport) -> RedactedCommunityPayload {
    RedactedCommunityPayload {
        scoring_version: report.scoring_version.clone(),
        profile: report.profile,
        total_score: report.total_score,
        suite_scores: report
            .suites
            .iter()
            .map(|suite| (suite.key.clone(), suite.score))
            .collect(),
        cpu_model: report.hardware.cpu.name.clone(),
        gpu_model: report
            .hardware
            .gpus
            .first()
            .map(|gpu| gpu.name.clone())
            .unwrap_or_else(|| "Unknown".into()),
        memory_gib: report.hardware.memory.total_bytes as f64 / 1_073_741_824.0,
        os: report.hardware.os.clone(),
    }
}

pub fn preview_upload(report: &BenchmarkReport) -> Result<RedactedCommunityPayload, String> {
    let settings = load_settings();
    if !settings.opt_in {
        return Err("Community sharing is not enabled.".into());
    }
    if settings
        .endpoint
        .as_ref()
        .map(|s| s.trim())
        .unwrap_or("")
        .is_empty()
    {
        return Err("No community endpoint configured. This release is API-ready only.".into());
    }
    let _timeout = Duration::from_secs(10);
    Ok(redact_report(report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    #[test]
    fn redaction_removes_hostname() {
        let report = BenchmarkReport {
            id: "x".into(),
            scoring_version: "2.0.0".into(),
            profile: BenchmarkProfile::Standard,
            generated_at_unix_ms: 1,
            duration_ms: 1,
            hardware: HardwareInfo {
                os: "Linux".into(),
                hostname: "secret-host".into(),
                cpu: CpuInfo {
                    name: "CPU".into(),
                    physical_cores: 4,
                    logical_cores: 8,
                    frequency_mhz: 1,
                },
                memory: MemoryInfo {
                    total_bytes: 8 << 30,
                    available_bytes: 4 << 30,
                },
                gpus: vec![],
                disks: vec![],
            },
            environment: RunEnvironment {
                os: "Linux".into(),
                cpu_usage_percent: 1.0,
                available_memory_bytes: 1,
                total_memory_bytes: 1,
                thermal_celsius: None,
                warnings: vec![],
            },
            suites: vec![],
            components: vec![],
            total_score: Some(1000.0),
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
        };
        let payload = redact_report(&report);
        let encoded = serde_json::to_string(&payload).unwrap();
        assert!(!encoded.contains("secret-host"));
        assert_eq!(payload.scoring_version, "2.0.0");
    }
}
