use super::runner::{hash_bytes, timed_throughput};
use crate::models::BenchmarkProfile;
use image::{imageops, Rgba, RgbaImage};
use std::sync::atomic::{AtomicBool, Ordering};

fn scale(profile: &BenchmarkProfile) -> u32 {
    match profile {
        BenchmarkProfile::Standard => 1,
        BenchmarkProfile::Extended => 2,
    }
}

fn make_source(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, ((x + y) % 256) as u8, 255])
    })
}

pub fn image_pipeline(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let factor = scale(profile);
    let width = 1280 * factor;
    let height = 720 * factor;
    let source = make_source(width, height);
    let megapixels = (width * height) as f64 / 1_000_000.0;
    let hash = hash_bytes(source.as_raw());
    let throughput = timed_throughput(cancelled, megapixels, || {
        if cancelled.load(Ordering::Relaxed) {
            return false;
        }
        let resized = imageops::resize(
            &source,
            width / 2,
            height / 2,
            imageops::FilterType::Triangle,
        );
        let blurred = imageops::blur(&resized, 1.2);
        let mut composited = blurred.clone();
        for (x, y, pixel) in composited.enumerate_pixels_mut() {
            let base = blurred.get_pixel(x, y);
            let overlay = source.get_pixel(x * 2, y * 2);
            *pixel = Rgba([
                ((base[0] as u16 + overlay[0] as u16) / 2) as u8,
                ((base[1] as u16 + overlay[1] as u16) / 2) as u8,
                ((base[2] as u16 + overlay[2] as u16) / 2) as u8,
                255,
            ]);
        }
        let mut encoded = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut encoded);
        image::DynamicImage::ImageRgba8(composited)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .is_ok()
            && !cancelled.load(Ordering::Relaxed)
    })
    .ok_or_else(|| "Image pipeline cancelled".to_string())?;
    Ok((throughput, hash))
}

pub fn cpu_render(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let factor = scale(profile);
    let width = 640 * factor;
    let height = 360 * factor;
    let megapixels = (width * height) as f64 / 1_000_000.0;
    let mut frame = vec![0_u8; (width * height * 4) as usize];
    let hash_seed = hash_bytes(&[width as u8, height as u8, factor as u8]);
    let throughput = timed_throughput(cancelled, megapixels, || {
        for y in 0..height {
            if y % 32 == 0 && cancelled.load(Ordering::Relaxed) {
                return false;
            }
            for x in 0..width {
                let fx = x as f32 / width as f32;
                let fy = y as f32 / height as f32;
                let dx = fx - 0.5;
                let dy = fy - 0.5;
                let dist = (dx * dx + dy * dy).sqrt();
                let shade = ((1.0 - dist).max(0.0) * 255.0) as u8;
                let idx = ((y * width + x) * 4) as usize;
                frame[idx] = shade;
                frame[idx + 1] = (shade as f32 * fx) as u8;
                frame[idx + 2] = (shade as f32 * fy) as u8;
                frame[idx + 3] = 255;
            }
        }
        true
    })
    .ok_or_else(|| "CPU render cancelled".to_string())?;
    let hash = hash_bytes(&frame);
    let _ = hash_seed;
    Ok((throughput, hash))
}
