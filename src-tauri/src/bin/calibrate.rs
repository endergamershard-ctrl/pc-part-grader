//! Developer calibration helper.
//! Runs selected workloads repeatedly and prints anonymous median samples
//! suitable for reviewing baseline reference values.
//!
//! Usage:
//!   cargo run --manifest-path src-tauri/Cargo.toml --bin calibrate -- standard

use pc_part_grader_lib::calibrate_main;

fn main() {
    if let Err(error) = calibrate_main() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
