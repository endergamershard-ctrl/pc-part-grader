use crate::models::{CpuInfo, DiskInfo, GpuInfo, HardwareInfo, MemoryInfo};
use sysinfo::{Disks, System};

pub fn detect() -> HardwareInfo {
    let mut system = System::new_all();
    system.refresh_all();

    let cpus = system.cpus();
    let cpu = CpuInfo {
        name: cpus
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "Unknown CPU".to_string()),
        physical_cores: System::physical_core_count().unwrap_or(cpus.len()),
        logical_cores: cpus.len(),
        frequency_mhz: cpus.iter().map(|cpu| cpu.frequency()).max().unwrap_or(0),
    };

    let memory = MemoryInfo {
        total_bytes: system.total_memory(),
        available_bytes: system.available_memory(),
    };

    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| DiskInfo {
            name: disk.name().to_string_lossy().into_owned(),
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            total_bytes: disk.total_space(),
            available_bytes: disk.available_space(),
            removable: disk.is_removable(),
        })
        .collect();

    HardwareInfo {
        os: format!(
            "{} {}",
            System::name().unwrap_or_else(|| std::env::consts::OS.to_string()),
            System::os_version().unwrap_or_default()
        )
        .trim()
        .to_string(),
        hostname: System::host_name().unwrap_or_else(|| "Local PC".to_string()),
        cpu,
        memory,
        gpus: detect_gpus(),
        disks,
    }
}

fn detect_gpus() -> Vec<GpuInfo> {
    let instance = wgpu::Instance::default();
    pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
        .into_iter()
        .map(|adapter| {
            let info = adapter.get_info();
            GpuInfo {
                name: info.name,
                vendor: vendor_name(info.vendor).to_string(),
                backend: format!("{:?}", info.backend),
                device_type: format!("{:?}", info.device_type),
            }
        })
        .collect()
}

fn vendor_name(vendor: u32) -> &'static str {
    match vendor {
        0x1002 | 0x1022 => "AMD",
        0x10de => "NVIDIA",
        0x8086 => "Intel",
        0x13b5 => "ARM",
        0x5143 => "Qualcomm",
        0x106b => "Apple",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::vendor_name;

    #[test]
    fn maps_common_gpu_vendors() {
        assert_eq!(vendor_name(0x10de), "NVIDIA");
        assert_eq!(vendor_name(0x8086), "Intel");
        assert_eq!(vendor_name(7), "Unknown");
    }
}
