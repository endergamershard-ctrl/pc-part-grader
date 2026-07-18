use super::runner::{hash_bytes, timed_throughput};
use crate::models::BenchmarkProfile;
use flate2::{write::GzEncoder, Compression};
use rayon::prelude::*;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

fn scale(profile: &BenchmarkProfile) -> usize {
    match profile {
        BenchmarkProfile::Standard => 1,
        BenchmarkProfile::Extended => 3,
    }
}

pub fn spreadsheet(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let rows = 120_000 * scale(profile);
    let cols = 16usize;
    let cells = rows * cols;
    let mut grid = vec![0.0_f64; cells];
    for (i, cell) in grid.iter_mut().enumerate() {
        *cell = ((i % 997) as f64) * 0.37 + 1.0;
    }
    let hash = hash_bytes(bytemuck::cast_slice(&grid));
    let throughput = timed_throughput(cancelled, cells as f64 / 1_000_000.0, || {
        let mut acc = 0.0;
        for row in 0..rows {
            if row % 2048 == 0 && cancelled.load(Ordering::Relaxed) {
                return false;
            }
            let mut sum = 0.0;
            let mut product = 1.0;
            for col in 0..cols {
                let value = grid[row * cols + col];
                sum += value;
                product *= value.sqrt().max(1.0);
            }
            acc += sum / cols as f64 + (product.ln().abs() / cols as f64);
        }
        acc.is_finite()
    })
    .ok_or_else(|| "Spreadsheet cancelled".to_string())?;
    Ok((throughput, hash))
}

pub fn sort_aggregate(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let rows = 500_000 * scale(profile);
    let mut data: Vec<(u32, f64)> = (0..rows as u32)
        .map(|i| (i.wrapping_mul(2654435761) % 10_000, (i % 997) as f64))
        .collect();
    let hash = hash_bytes(bytemuck::cast_slice(
        &data
            .iter()
            .map(|(k, v)| (*k as u64) ^ v.to_bits())
            .collect::<Vec<_>>(),
    ));
    let throughput = timed_throughput(cancelled, rows as f64 / 1_000_000.0, || {
        if cancelled.load(Ordering::Relaxed) {
            return false;
        }
        data.par_sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let mut groups = 0u64;
        let mut last = u32::MAX;
        let mut sum = 0.0;
        for (key, value) in &data {
            if *key != last {
                groups += 1;
                last = *key;
            }
            sum += value;
        }
        groups > 0 && sum.is_finite() && !cancelled.load(Ordering::Relaxed)
    })
    .ok_or_else(|| "Sort/aggregate cancelled".to_string())?;
    Ok((throughput, hash))
}

pub fn document_transform(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let paragraphs = 20_000 * scale(profile);
    let mut document = String::with_capacity(paragraphs * 80);
    for i in 0..paragraphs {
        document.push_str(&format!(
            "Paragraph {i}: The quick brown fox jumps over the lazy dog. Token-{}.\n",
            i % 50
        ));
    }
    let bytes = document.len() as f64;
    let hash = hash_bytes(document.as_bytes());
    let throughput = timed_throughput(cancelled, bytes / 1_000_000.0, || {
        let transformed: String = document
            .lines()
            .enumerate()
            .map(|(idx, line)| {
                let upper = line.to_uppercase();
                format!("{idx:05}|{}", upper.replace("TOKEN", "MARK"))
            })
            .collect::<Vec<_>>()
            .join("\n");
        !cancelled.load(Ordering::Relaxed) && transformed.len() > document.len() / 2
    })
    .ok_or_else(|| "Document transform cancelled".to_string())?;
    Ok((throughput, hash))
}

pub fn archive(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let files = 80 * scale(profile);
    let file_size = 64 * 1024;
    let mut blobs = Vec::with_capacity(files);
    for i in 0..files {
        let mut blob = vec![0_u8; file_size];
        for (offset, byte) in blob.iter_mut().enumerate() {
            *byte = ((offset + i * 13) % 255) as u8;
        }
        blobs.push(blob);
    }
    let total = (files * file_size) as f64;
    let hash = hash_bytes(&blobs.concat());
    let throughput = timed_throughput(cancelled, total / 1_000_000.0, || {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        for blob in &blobs {
            if cancelled.load(Ordering::Relaxed) {
                return false;
            }
            if encoder.write_all(blob).is_err() {
                return false;
            }
        }
        encoder.finish().is_ok()
    })
    .ok_or_else(|| "Archive cancelled".to_string())?;
    Ok((throughput, hash))
}
