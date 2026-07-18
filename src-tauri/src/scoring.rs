use crate::models::{
    BenchmarkProfile, BenchmarkReport, ComponentScore, HardwareInfo, LocalComparison,
    RunEnvironment, SuiteScore, WorkloadResult,
};
use serde::Deserialize;
use std::collections::HashMap;

pub const SCORING_VERSION: &str = "2.0.0";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaselineFile {
    scoring_version: String,
    reference_note: String,
    workloads: HashMap<String, f64>,
    tiers: Vec<BaselineTier>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaselineTier {
    name: String,
    min_total: f64,
}

fn baselines() -> BaselineFile {
    serde_json::from_str(include_str!("../baselines/v2.json")).unwrap_or_else(|_| BaselineFile {
        scoring_version: SCORING_VERSION.into(),
        reference_note: "Fallback local reference values.".into(),
        workloads: default_references(),
        tiers: vec![
            BaselineTier {
                name: "Entry".into(),
                min_total: 0.0,
            },
            BaselineTier {
                name: "Mainstream".into(),
                min_total: 900.0,
            },
            BaselineTier {
                name: "Enthusiast".into(),
                min_total: 1400.0,
            },
            BaselineTier {
                name: "Extreme".into(),
                min_total: 2000.0,
            },
        ],
    })
}

fn default_references() -> HashMap<String, f64> {
    HashMap::from([
        ("json_parse".into(), 180.0),
        ("text_search".into(), 900.0),
        ("compression".into(), 220.0),
        ("small_files".into(), 1200.0),
        ("spreadsheet".into(), 45.0),
        ("sort_aggregate".into(), 80.0),
        ("document_transform".into(), 250.0),
        ("archive".into(), 180.0),
        ("image_pipeline".into(), 18.0),
        ("cpu_render".into(), 35.0),
        ("gpu_compute".into(), 120.0),
        ("gpu_offscreen".into(), 900.0),
        ("gpu_transfer".into(), 180.0),
        ("storage_sequential".into(), 1200.0),
        ("storage_random".into(), 8000.0),
    ])
}

#[allow(clippy::too_many_arguments)]
pub fn build_report(
    id: String,
    profile: BenchmarkProfile,
    hardware: HardwareInfo,
    environment: RunEnvironment,
    mut suites: Vec<SuiteScore>,
    duration_ms: u128,
    cancelled: bool,
    mut notes: Vec<String>,
) -> BenchmarkReport {
    let baseline = baselines();
    if baseline.scoring_version != SCORING_VERSION {
        notes.push(format!(
            "Baseline file version {} differs from scoring model {}.",
            baseline.scoring_version, SCORING_VERSION
        ));
    }

    for suite in &mut suites {
        for workload in &mut suite.workloads {
            score_workload(workload, &baseline.workloads);
        }
        finalize_suite(suite);
    }

    let total_score = weighted_geometric_mean(
        suites
            .iter()
            .filter_map(|suite| suite.score.map(|score| (score, suite.weight))),
    );
    let total_grade = total_score.map(score_to_grade);
    let letter_grade = letter_from_grade(total_grade);
    let components = suites
        .iter()
        .map(|suite| ComponentScore {
            key: suite.key.clone(),
            label: suite.label.clone(),
            score: suite.grade,
            weight: suite.weight,
            confidence: suite.reliability.clone(),
            explanation: suite
                .bottleneck
                .clone()
                .unwrap_or_else(|| format!("{} suite score from valid workloads.", suite.label)),
        })
        .collect();

    let reference_tier = total_score.and_then(|score| {
        baseline
            .tiers
            .iter()
            .rev()
            .find(|tier| score >= tier.min_total)
            .map(|tier| tier.name.clone())
    });

    if suites.iter().any(|suite| suite.score.is_none()) {
        notes.push(
            "Overall score uses only valid suites/workloads; unreliable results are excluded."
                .into(),
        );
    }

    BenchmarkReport {
        id,
        scoring_version: SCORING_VERSION.into(),
        profile,
        generated_at_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        duration_ms,
        hardware,
        environment,
        suites,
        components,
        total_score,
        total_grade,
        letter_grade,
        comparison: LocalComparison {
            previous_total: None,
            best_total: None,
            delta_vs_previous: None,
            delta_vs_best: None,
            reference_tier,
            reference_note: baseline.reference_note,
        },
        notes,
        cancelled,
    }
}

fn score_workload(workload: &mut WorkloadResult, references: &HashMap<String, f64>) {
    let Some(stats) = &workload.stats else {
        workload.valid = false;
        return;
    };
    if !workload.valid {
        return;
    }
    let reference = references
        .get(&workload.key)
        .copied()
        .unwrap_or(stats.median.max(1.0));
    let ratio = if workload.higher_is_better {
        stats.median / reference.max(f64::EPSILON)
    } else {
        reference.max(f64::EPSILON) / stats.median.max(f64::EPSILON)
    };
    let score = (ratio * 1000.0).max(1.0);
    workload.score = Some(round1(score));
    workload.grade = Some(score_to_grade(score));
}

fn finalize_suite(suite: &mut SuiteScore) {
    let scored: Vec<(f64, f64)> = suite
        .workloads
        .iter()
        .filter(|w| w.valid)
        .filter_map(|w| w.score.map(|score| (score, w.weight)))
        .collect();
    suite.score = weighted_geometric_mean(scored);
    suite.grade = suite.score.map(score_to_grade);
    suite.reliability = suite_reliability(&suite.workloads);
    suite.bottleneck = suite
        .workloads
        .iter()
        .filter(|w| w.valid)
        .min_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|w| format!("Bottleneck: {} ({:.0})", w.label, w.score.unwrap_or(0.0)));
}

fn suite_reliability(workloads: &[WorkloadResult]) -> String {
    if workloads.iter().all(|w| w.valid) {
        if workloads
            .iter()
            .all(|w| matches!(w.reliability.as_str(), "excellent" | "good"))
        {
            "high".into()
        } else {
            "moderate".into()
        }
    } else if workloads.iter().any(|w| w.valid) {
        "partial".into()
    } else {
        "unavailable".into()
    }
}

pub fn weighted_geometric_mean<I>(values: I) -> Option<f64>
where
    I: IntoIterator<Item = (f64, f64)>,
{
    let mut log_sum = 0.0;
    let mut weight_sum = 0.0;
    for (value, weight) in values {
        if value <= 0.0 || weight <= 0.0 {
            continue;
        }
        log_sum += value.ln() * weight;
        weight_sum += weight;
    }
    (weight_sum > 0.0).then(|| round1((log_sum / weight_sum).exp()))
}

pub fn score_to_grade(score: f64) -> f64 {
    // Map the 1000-point mainstream reference to ~75, asymptote toward 100.
    // 720 ≈ 1000 / ln(4), which puts exp(-1000/720) at 0.25.
    let grade = 100.0 * (1.0 - (-score / 720.0).exp());
    round1(grade.clamp(0.0, 100.0))
}

pub fn letter_from_grade(grade: Option<f64>) -> String {
    match grade {
        Some(g) if g >= 95.0 => "A+".into(),
        Some(g) if g >= 90.0 => "A".into(),
        Some(g) if g >= 80.0 => "B".into(),
        Some(g) if g >= 70.0 => "C".into(),
        Some(g) if g >= 60.0 => "D".into(),
        Some(_) => "F".into(),
        None => "N/A".into(),
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

pub fn attach_history_comparison(
    report: &mut BenchmarkReport,
    previous_total: Option<f64>,
    best_total: Option<f64>,
) {
    report.comparison.previous_total = previous_total;
    report.comparison.best_total = best_total;
    report.comparison.delta_vs_previous = match (report.total_score, previous_total) {
        (Some(current), Some(previous)) => Some(round1(current - previous)),
        _ => None,
    };
    report.comparison.delta_vs_best = match (report.total_score, best_total) {
        (Some(current), Some(best)) => Some(round1(current - best)),
        _ => None,
    };
}

#[cfg(test)]
mod tests {
    use super::{letter_from_grade, score_to_grade, weighted_geometric_mean};

    #[test]
    fn geometric_mean_penalizes_weak_link() {
        let balanced = weighted_geometric_mean([(1000.0, 1.0), (1000.0, 1.0)]).unwrap();
        let weak = weighted_geometric_mean([(1000.0, 1.0), (250.0, 1.0)]).unwrap();
        assert!(balanced > weak);
    }

    #[test]
    fn grades_and_letters_map_sensibly() {
        let mainstream = score_to_grade(1000.0);
        assert!((70.0..=80.0).contains(&mainstream));
        assert!(score_to_grade(400.0) < 50.0);
        assert_eq!(letter_from_grade(Some(96.0)), "A+");
        assert_eq!(letter_from_grade(None), "N/A");
    }
}
