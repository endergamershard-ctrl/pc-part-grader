use super::runner::hash_bytes;
use crate::models::BenchmarkProfile;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

fn scale(profile: &BenchmarkProfile) -> u64 {
    match profile {
        BenchmarkProfile::Standard => 1,
        BenchmarkProfile::Extended => 2,
    }
}

async fn open_device() -> Result<(wgpu::Device, wgpu::Queue, wgpu::Adapter), String> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|error| format!("No compatible GPU adapter: {error}"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("PC Part Grader graphics device"),
            ..Default::default()
        })
        .await
        .map_err(|error| format!("Could not open GPU device: {error}"))?;
    Ok((device, queue, adapter))
}

pub fn gpu_compute(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    if cancelled.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    pollster::block_on(gpu_compute_async(profile, cancelled))
}

async fn gpu_compute_async(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let (device, queue, _) = open_device().await?;
    let elements = (1 << 20) * scale(profile);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("compute"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
            @group(0) @binding(0) var<storage, read_write> values: array<f32>;
            @compute @workgroup_size(64)
            fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                let i = id.x;
                if (i >= arrayLength(&values)) { return; }
                var v = values[i];
                for (var k = 0u; k < 32u; k++) {
                    v = v * 1.0001 + 0.0001;
                    v = sqrt(abs(v)) + sin(v);
                }
                values[i] = v;
            }
            "#
            .into(),
        ),
    });
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("values"),
        size: elements * 4,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let zeros = vec![1.0_f32; elements as usize];
    queue.write_buffer(&buffer, 0, bytemuck::cast_slice(&zeros));
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(elements.div_ceil(64) as u32, 1, 1);
    }
    if cancelled.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    let started = Instant::now();
    let submission = queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(20)),
        })
        .map_err(|e| format!("GPU compute timed out: {e}"))?;
    // Rough FLOPS estimate: elements * 32 iters * ~6 ops
    let flops = elements as f64 * 32.0 * 6.0;
    let gflops = flops / started.elapsed().as_secs_f64() / 1_000_000_000.0;
    Ok((gflops, hash_bytes(&elements.to_le_bytes())))
}

pub fn gpu_offscreen(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    if cancelled.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    pollster::block_on(gpu_offscreen_async(profile, cancelled))
}

async fn gpu_offscreen_async(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let (device, queue, _) = open_device().await?;
    let size = 512 * scale(profile) as u32;
    let frames = 60 * scale(profile) as u32;
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("offscreen"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
            @vertex
            fn vs(@builtin(vertex_index) idx: u32) -> @builtin(position) vec4<f32> {
                var pos = array<vec2<f32>, 3>(
                    vec2<f32>(-1.0, -1.0),
                    vec2<f32>(3.0, -1.0),
                    vec2<f32>(-1.0, 3.0)
                );
                return vec4<f32>(pos[idx], 0.0, 1.0);
            }
            @fragment
            fn fs(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
                let uv = pos.xy / 512.0;
                return vec4<f32>(uv.x, uv.y, 0.25, 1.0);
            }
            "#
            .into(),
        ),
    });
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("offscreen-pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    for _ in 0..frames {
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..3, 0..1);
    }
    let started = Instant::now();
    let submission = queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(20)),
        })
        .map_err(|e| format!("GPU offscreen timed out: {e}"))?;
    let fps = frames as f64 / started.elapsed().as_secs_f64();
    Ok((fps, hash_bytes(&size.to_le_bytes())))
}

pub fn gpu_transfer(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    if cancelled.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }
    pollster::block_on(gpu_transfer_async(profile, cancelled))
}

async fn gpu_transfer_async(
    profile: &BenchmarkProfile,
    cancelled: &AtomicBool,
) -> Result<(f64, String), String> {
    let (device, queue, _) = open_device().await?;
    let bytes = (32 * 1024 * 1024) * scale(profile);
    let passes = 24u64;
    let source = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("src"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let destination = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("dst"),
        size: bytes,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    for pass in 0..passes {
        if cancelled.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        if pass % 2 == 0 {
            encoder.copy_buffer_to_buffer(&source, 0, &destination, 0, bytes);
        } else {
            encoder.copy_buffer_to_buffer(&destination, 0, &source, 0, bytes);
        }
    }
    let started = Instant::now();
    let submission = queue.submit(Some(encoder.finish()));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(15)),
        })
        .map_err(|e| format!("GPU transfer timed out: {e}"))?;
    let gbps = (bytes * passes) as f64 / started.elapsed().as_secs_f64() / 1_000_000_000.0;
    Ok((gbps, hash_bytes(&bytes.to_le_bytes())))
}
