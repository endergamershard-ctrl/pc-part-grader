use crate::models::SampleStats;

pub fn summarize(samples: &[f64]) -> Option<SampleStats> {
    if samples.is_empty() {
        return None;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if sorted.len() % 2 == 1 {
        sorted[sorted.len() / 2]
    } else {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    };
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let variance = if samples.len() > 1 {
        samples
            .iter()
            .map(|value| {
                let delta = value - mean;
                delta * delta
            })
            .sum::<f64>()
            / (samples.len() as f64 - 1.0)
    } else {
        0.0
    };
    let std_dev = variance.sqrt();
    let coefficient_of_variation = if mean.abs() > f64::EPSILON {
        (std_dev / mean.abs()) * 100.0
    } else {
        0.0
    };
    Some(SampleStats {
        samples: samples.to_vec(),
        median,
        mean,
        std_dev,
        coefficient_of_variation,
        reliability: reliability_label(coefficient_of_variation).into(),
    })
}

pub fn reliability_label(cv: f64) -> &'static str {
    if cv < 10.0 {
        "excellent"
    } else if cv < 20.0 {
        "good"
    } else if cv < 35.0 {
        "moderate"
    } else {
        "poor"
    }
}

pub fn is_reliable(stats: &SampleStats) -> bool {
    matches!(
        stats.reliability.as_str(),
        "excellent" | "good" | "moderate"
    )
}

#[cfg(test)]
mod tests {
    use super::{reliability_label, summarize};

    #[test]
    fn median_and_cv_are_computed() {
        let stats = summarize(&[10.0, 12.0, 11.0]).unwrap();
        assert_eq!(stats.median, 11.0);
        assert!(stats.coefficient_of_variation < 15.0);
        assert_eq!(reliability_label(5.0), "excellent");
    }
}
