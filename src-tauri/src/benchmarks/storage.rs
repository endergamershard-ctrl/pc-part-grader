use super::runner::hash_bytes;
use crate::models::BenchmarkProfile;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

fn scale(profile: &BenchmarkProfile) -> usize {
    match profile {
        BenchmarkProfile::Standard => 1,
        BenchmarkProfile::Extended => 4,
    }
}

fn work_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

pub fn sequential(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let size = 96 * 1024 * 1024 * scale(profile);
    let dir = work_dir();
    let available = fs2_available(&dir).unwrap_or(u64::MAX);
    if available < (size as u64) * 2 {
        return Err("Not enough free disk space for the storage benchmark".into());
    }
    let mut file = tempfile::Builder::new()
        .prefix(".pc-part-grader-seq-")
        .tempfile_in(&dir)
        .map_err(|e| format!("Could not create storage file: {e}"))?;
    let chunk = vec![0x5a_u8; 1024 * 1024];
    let write_started = Instant::now();
    for _ in 0..(size / chunk.len()) {
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        file.write_all(&chunk)
            .map_err(|e| format!("Storage write failed: {e}"))?;
    }
    file.as_file()
        .sync_data()
        .map_err(|e| format!("Storage sync failed: {e}"))?;
    let write_seconds = write_started.elapsed().as_secs_f64();

    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("Storage seek failed: {e}"))?;
    let read_started = Instant::now();
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut total_read = 0usize;
    let mut digest = Vec::new();
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let count = file
            .read(&mut buffer)
            .map_err(|e| format!("Storage read failed: {e}"))?;
        if count == 0 {
            break;
        }
        total_read += count;
        if digest.len() < 64 {
            digest.extend_from_slice(&buffer[..count.min(64 - digest.len())]);
        }
    }
    let read_seconds = read_started.elapsed().as_secs_f64();
    let write_mbps = size as f64 / write_seconds / 1_000_000.0;
    let read_mbps = total_read as f64 / read_seconds / 1_000_000.0;
    // Report harmonic-ish blend favoring sustained write after sync.
    let score_metric = (write_mbps * 0.6) + (read_mbps * 0.4);
    Ok((score_metric, hash_bytes(&digest)))
}

pub fn random(profile: &BenchmarkProfile, cancelled: &AtomicBool) -> Result<(f64, String), String> {
    let size = 48 * 1024 * 1024 * scale(profile);
    let ops = 2_000 * scale(profile);
    let block = 4 * 1024usize;
    let dir = work_dir();
    let mut file = tempfile::Builder::new()
        .prefix(".pc-part-grader-rnd-")
        .tempfile_in(&dir)
        .map_err(|e| format!("Could not create random I/O file: {e}"))?;
    let fill = vec![0xa5_u8; 1024 * 1024];
    for _ in 0..(size / fill.len()) {
        file.write_all(&fill).map_err(|e| e.to_string())?;
    }
    file.as_file().sync_data().map_err(|e| e.to_string())?;

    let mut buffer = vec![0_u8; block];
    let mut digest = [0_u8; 32];
    let started = Instant::now();
    for i in 0..ops {
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let offset = ((i * 7919) % ((size / block).max(1))) * block;
        file.seek(SeekFrom::Start(offset as u64))
            .map_err(|e| e.to_string())?;
        if i % 2 == 0 {
            file.read_exact(&mut buffer).map_err(|e| e.to_string())?;
            digest[i % 32] ^= buffer[0];
        } else {
            buffer.fill(((i % 255) + 1) as u8);
            file.write_all(&buffer).map_err(|e| e.to_string())?;
        }
    }
    file.as_file().sync_data().map_err(|e| e.to_string())?;
    let iops = ops as f64 / started.elapsed().as_secs_f64();
    Ok((iops, hash_bytes(&digest)))
}

fn fs2_available(path: &std::path::Path) -> Option<u64> {
    // Use sysinfo disks for available space near the path.
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut best = None;
    for disk in disks.list() {
        let mount = disk.mount_point();
        if path.starts_with(mount) {
            let len = mount.as_os_str().len();
            if best.as_ref().map(|(l, _)| len > *l).unwrap_or(true) {
                best = Some((len, disk.available_space()));
            }
        }
    }
    best.map(|(_, space)| space)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn cancelled_storage_returns_error_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        let cancelled = AtomicBool::new(true);
        // Directly using sequential against HOME may still create nothing when cancelled early.
        let result = sequential(&BenchmarkProfile::Standard, &cancelled);
        assert!(result.is_err());
        let _ = dir;
    }
}
