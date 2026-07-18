use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BenchmarkProfile {
    Standard,
    Extended,
}

impl BenchmarkProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Extended => "extended",
        }
    }

    pub fn estimated_seconds(self) -> u32 {
        match self {
            Self::Standard => 240,
            Self::Extended => 780,
        }
    }

    pub fn warmups(self) -> usize {
        match self {
            Self::Standard => 1,
            Self::Extended => 2,
        }
    }

    pub fn samples(self) -> usize {
        match self {
            Self::Standard => 3,
            Self::Extended => 7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub os: String,
    pub hostname: String,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpus: Vec<GpuInfo>,
    pub disks: Vec<DiskInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub name: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub frequency_mhz: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
    pub backend: String,
    pub device_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub removable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunEnvironment {
    pub os: String,
    pub cpu_usage_percent: f32,
    pub available_memory_bytes: u64,
    pub total_memory_bytes: u64,
    pub thermal_celsius: Option<f32>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleStats {
    pub samples: Vec<f64>,
    pub median: f64,
    pub mean: f64,
    pub std_dev: f64,
    pub coefficient_of_variation: f64,
    pub reliability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadResult {
    pub key: String,
    pub label: String,
    pub suite: String,
    pub unit: String,
    pub higher_is_better: bool,
    pub stats: Option<SampleStats>,
    pub score: Option<f64>,
    pub grade: Option<f64>,
    pub weight: f64,
    pub valid: bool,
    pub reliability: String,
    pub explanation: String,
    pub output_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuiteScore {
    pub key: String,
    pub label: String,
    pub score: Option<f64>,
    pub grade: Option<f64>,
    pub weight: f64,
    pub reliability: String,
    pub bottleneck: Option<String>,
    pub workloads: Vec<WorkloadResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentScore {
    pub key: String,
    pub label: String,
    pub score: Option<f64>,
    pub weight: f64,
    pub confidence: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalComparison {
    pub previous_total: Option<f64>,
    pub best_total: Option<f64>,
    pub delta_vs_previous: Option<f64>,
    pub delta_vs_best: Option<f64>,
    pub reference_tier: Option<String>,
    pub reference_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkReport {
    pub id: String,
    pub scoring_version: String,
    pub profile: BenchmarkProfile,
    pub generated_at_unix_ms: u128,
    pub duration_ms: u128,
    pub hardware: HardwareInfo,
    pub environment: RunEnvironment,
    pub suites: Vec<SuiteScore>,
    pub components: Vec<ComponentScore>,
    pub total_score: Option<f64>,
    pub total_grade: Option<f64>,
    pub letter_grade: String,
    pub comparison: LocalComparison,
    pub notes: Vec<String>,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummary {
    pub id: String,
    pub profile: BenchmarkProfile,
    pub generated_at_unix_ms: u128,
    pub total_score: Option<f64>,
    pub letter_grade: String,
    pub cpu_name: String,
    pub gpu_name: String,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkProgress {
    pub stage: String,
    pub suite: String,
    pub workload: String,
    pub percent: u8,
    pub message: String,
    pub elapsed_ms: u128,
    pub estimated_remaining_ms: Option<u128>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunitySettings {
    pub opt_in: bool,
    pub endpoint: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedactedCommunityPayload {
    pub scoring_version: String,
    pub profile: BenchmarkProfile,
    pub total_score: Option<f64>,
    pub suite_scores: Vec<(String, Option<f64>)>,
    pub cpu_model: String,
    pub gpu_model: String,
    pub memory_gib: f64,
    pub os: String,
}
