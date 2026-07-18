use super::runner::{hash_bytes, timed_throughput};
use crate::models::BenchmarkProfile;
use flate2::{write::GzEncoder, Compression};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

fn scale(profile: &BenchmarkProfile) -> usize {
    match profile {
        BenchmarkProfile::Standard => 1,
        BenchmarkProfile::Extended => 3,
    }
}

pub fn json_parse(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let rows = 40_000 * scale(profile);
    let mut payload = String::with_capacity(rows * 48);
    payload.push('[');
    for i in 0..rows {
        if i > 0 {
            payload.push(',');
        }
        payload.push_str(&format!(
            "{{\"id\":{i},\"score\":{},\"tag\":\"item-{}\"}}",
            (i % 1000) as f64 / 10.0,
            i % 64
        ));
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
    }
    payload.push(']');
    let bytes = payload.len() as f64;
    let hash = hash_bytes(payload.as_bytes());
    let throughput = timed_throughput(cancelled, bytes / 1_000_000.0, || {
        let value: serde_json::Value =
            serde_json::from_str(&payload).expect("generated json must parse");
        !cancelled.load(Ordering::Relaxed) && value.as_array().map(|a| a.len()).unwrap_or(0) == rows
    })
    .ok_or_else(|| "JSON parse cancelled".to_string())?;
    Ok((throughput, hash))
}

pub fn text_search(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let lines = 80_000 * scale(profile);
    let corpus: String = (0..lines)
        .map(|i| format!("record {i} alpha beta gamma token-{}\n", i % 97))
        .collect();
    let bytes = corpus.len() as f64;
    let needle = "token-42";
    let hash = hash_bytes(corpus.as_bytes());
    let throughput = timed_throughput(cancelled, bytes / 1_000_000.0, || {
        let count = corpus.matches(needle).count();
        !cancelled.load(Ordering::Relaxed) && count > 0
    })
    .ok_or_else(|| "Text search cancelled".to_string())?;
    Ok((throughput, hash))
}

pub fn compression(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let size = 12 * 1024 * 1024 * scale(profile);
    let mut data = vec![0_u8; size];
    for (index, byte) in data.iter_mut().enumerate() {
        *byte = ((index * 17) % 251) as u8;
    }
    let hash = hash_bytes(&data);
    let throughput = timed_throughput(cancelled, size as f64 / 1_000_000.0, || {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        if encoder.write_all(&data).is_err() {
            return false;
        }
        encoder.finish().is_ok() && !cancelled.load(Ordering::Relaxed)
    })
    .ok_or_else(|| "Compression cancelled".to_string())?;
    Ok((throughput, hash))
}

pub fn small_files(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let count = 400 * scale(profile);
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let started = std::time::Instant::now();
    let mut digest = Vec::new();
    for i in 0..count {
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let path = dir.path().join(format!("f-{i}.bin"));
        let payload = format!("payload-{i}-{}", i * 13).into_bytes();
        std::fs::write(&path, &payload).map_err(|e| e.to_string())?;
        let read = std::fs::read(&path).map_err(|e| e.to_string())?;
        digest.extend_from_slice(&read);
        let _ = std::fs::remove_file(path);
    }
    let ops = count as f64 / started.elapsed().as_secs_f64();
    Ok((ops, hash_bytes(&digest)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn json_parse_produces_result() {
        let cancelled = AtomicBool::new(false);
        let (value, hash) = json_parse(&BenchmarkProfile::Standard, &cancelled).unwrap();
        assert!(value > 0.0);
        assert_eq!(hash.len(), 64);
    }
}
