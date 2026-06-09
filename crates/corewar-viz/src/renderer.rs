//! WebGPU renderer for core memory visualization.
//!
//! The renderer keeps enough state around to initialize and manage a GPU pipeline,
//! while the higher-level visualization state stays reusable for native and WASM frontends.

use std::fmt;

use bytemuck::{Pod, Zeroable};
use corewar_protocol::CycleEvent;
use tracing::{error, warn};
use wgpu::util::DeviceExt;

/// WGSL source for the fullscreen triangle vertex shader.
pub const GRID_VERTEX_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );

    var out: VertexOutput;
    out.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    return out;
}
"#;

/// WGSL source for the fragment shader that shades core cells by owner and heat.
pub const GRID_FRAGMENT_SHADER: &str = r#"
struct RendererUniforms {
    grid_width: u32,
    grid_height: u32,
    cell_size: f32,
    _padding: f32,
    background_color: vec4<f32>,
};

struct GpuCell {
    owner: u32,
    heat: f32,
    _padding0: vec2<u32>,
};

@group(0) @binding(0)
var<uniform> uniforms: RendererUniforms;

@group(0) @binding(1)
var<storage, read> cells: array<GpuCell>;

fn owner_color(owner: u32) -> vec3<f32> {
    let base = fract(f32(owner) * 0.61803398875);
    return vec3<f32>(
        fract(base + 0.00),
        fract(base + 0.33),
        fract(base + 0.66),
    );
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let cell_x = u32(floor(position.x / uniforms.cell_size));
    let cell_y = u32(floor(position.y / uniforms.cell_size));

    if (cell_x >= uniforms.grid_width || cell_y >= uniforms.grid_height) {
        return uniforms.background_color;
    }

    let index = cell_y * uniforms.grid_width + cell_x;
    let cell = cells[index];
    let line_x = step(0.96, fract(position.x / uniforms.cell_size));
    let line_y = step(0.96, fract(position.y / uniforms.cell_size));
    let line_mix = max(line_x, line_y) * 0.15;

    if (cell.owner == 0xffffffffu) {
        let idle_glow = vec3<f32>(0.08, 0.08, 0.12) * clamp(cell.heat, 0.0, 1.0);
        let base = uniforms.background_color.rgb + idle_glow;
        return vec4<f32>(base + line_mix, uniforms.background_color.a);
    }

    let base_color = owner_color(cell.owner);
    let heat = clamp(cell.heat, 0.0, 1.0);
    let shaded = mix(base_color * 0.35, base_color, heat);
    return vec4<f32>(shaded + line_mix, 1.0);
}
"#;

const UNOWNED_SENTINEL: u32 = u32::MAX;

/// Configuration for the renderer.
#[derive(Debug, Clone)]
pub struct RendererConfig {
    /// How many cells wide the grid should be.
    pub grid_width: u32,
    /// How many cells tall the grid should be.
    pub grid_height: u32,
    /// The cell size in screen pixels.
    pub cell_size: f32,
    /// Background color for unowned cells.
    pub background_color: [f32; 4],
    /// How quickly the heat map fades per frame (0.0 = instant, 1.0 = permanent).
    pub heat_decay: f32,
}

impl RendererConfig {
    /// Total number of cells in the rendered core grid.
    pub fn cell_count(&self) -> usize {
        self.grid_width as usize * self.grid_height as usize
    }

    /// Pixel dimensions for the configured render target.
    pub fn surface_size(&self) -> (u32, u32) {
        let width = (self.grid_width as f32 * self.cell_size).max(1.0).round() as u32;
        let height = (self.grid_height as f32 * self.cell_size).max(1.0).round() as u32;
        (width, height)
    }
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            grid_width: 100,
            grid_height: 80,
            cell_size: 8.0,
            background_color: [0.05, 0.05, 0.08, 1.0],
            heat_decay: 0.95,
        }
    }
}

/// Visual state for a single core address.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellState {
    pub owner: Option<u32>,
    pub heat: f32,
    pub last_access_cycle: u64,
}

impl Default for CellState {
    fn default() -> Self {
        Self {
            owner: None,
            heat: 0.0,
            last_access_cycle: 0,
        }
    }
}

/// CPU-side visual model of the core grid.
#[derive(Debug, Clone)]
pub struct CoreState {
    pub cells: Vec<CellState>,
    pub width: u32,
    pub height: u32,
    pub current_cycle: u64,
}

impl CoreState {
    pub fn new(width: u32, height: u32) -> Self {
        let len = width as usize * height as usize;
        Self {
            cells: vec![CellState::default(); len],
            width,
            height,
            current_cycle: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn decay_heat(&mut self, decay_factor: f32) {
        let factor = decay_factor.clamp(0.0, 1.0);
        for cell in &mut self.cells {
            cell.heat *= factor;
        }
    }

    pub fn apply_events(&mut self, events: &[CycleEvent]) {
        self.current_cycle = self.current_cycle.saturating_add(1);

        for event in events {
            match *event {
                CycleEvent::Write {
                    address,
                    warrior_id,
                }
                | CycleEvent::Execute {
                    address,
                    warrior_id,
                }
                | CycleEvent::ProcessCreated {
                    warrior_id,
                    address,
                } => self.touch_cell(address, Some(warrior_id)),
                CycleEvent::ProcessKilled { address, .. } => self.touch_cell(address, None),
            }
        }
    }

    fn touch_cell(&mut self, address: usize, owner: Option<u32>) {
        if let Some(cell) = self.cells.get_mut(address) {
            cell.owner = owner;
            cell.heat = 1.0;
            cell.last_access_cycle = self.current_cycle;
        }
    }

    fn as_gpu_cells(&self) -> Vec<GpuCell> {
        self.cells.iter().copied().map(GpuCell::from).collect()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct GpuCell {
    owner: u32,
    heat: f32,
    _padding: [u32; 2],
}

impl From<CellState> for GpuCell {
    fn from(value: CellState) -> Self {
        Self {
            owner: value.owner.unwrap_or(UNOWNED_SENTINEL),
            heat: value.heat,
            _padding: [0; 2],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct RendererUniforms {
    grid_width: u32,
    grid_height: u32,
    cell_size: f32,
    _padding: f32,
    background_color: [f32; 4],
}

impl RendererUniforms {
    fn from_config(config: &RendererConfig) -> Self {
        Self {
            grid_width: config.grid_width,
            grid_height: config.grid_height,
            cell_size: config.cell_size,
            _padding: 0.0,
            background_color: config.background_color,
        }
    }
}

#[derive(Debug)]
pub enum RendererInitError {
    SurfaceCreation(wgpu::CreateSurfaceError),
    AdapterUnavailable,
    DeviceRequest(wgpu::RequestDeviceError),
    SurfaceUnsupported,
}

impl fmt::Display for RendererInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceCreation(err) => write!(f, "failed to create WebGPU surface: {err}"),
            Self::AdapterUnavailable => write!(f, "failed to find a compatible WebGPU adapter"),
            Self::DeviceRequest(err) => write!(f, "failed to request WebGPU device: {err}"),
            Self::SurfaceUnsupported => {
                write!(
                    f,
                    "surface does not expose a supported presentation configuration"
                )
            }
        }
    }
}

impl std::error::Error for RendererInitError {}

/// WebGPU renderer and its GPU resources.
pub struct Renderer<'window> {
    pub config: RendererConfig,
    pub core_state: CoreState,
    pub instance: Option<wgpu::Instance>,
    pub surface: Option<wgpu::Surface<'window>>,
    pub device: Option<wgpu::Device>,
    pub queue: Option<wgpu::Queue>,
    pub surface_config: Option<wgpu::SurfaceConfiguration>,
    pub pipeline: Option<wgpu::RenderPipeline>,
    pub bind_group_layout: Option<wgpu::BindGroupLayout>,
    pub bind_group: Option<wgpu::BindGroup>,
    pub uniform_buffer: Option<wgpu::Buffer>,
    pub cell_buffer: Option<wgpu::Buffer>,
}

impl<'window> Renderer<'window> {
    pub async fn new<W>(window: W) -> Result<Self, RendererInitError>
    where
        W: Into<wgpu::SurfaceTarget<'window>>,
    {
        Self::with_config(window, RendererConfig::default()).await
    }

    pub async fn with_config<W>(
        window: W,
        config: RendererConfig,
    ) -> Result<Self, RendererInitError>
    where
        W: Into<wgpu::SurfaceTarget<'window>>,
    {
        let core_state = CoreState::new(config.grid_width, config.grid_height);
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window)
            .map_err(RendererInitError::SurfaceCreation)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(RendererInitError::AdapterUnavailable)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("corewar-viz-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(RendererInitError::DeviceRequest)?;

        let (surface_width, surface_height) = config.surface_size();
        let surface_config = surface
            .get_default_config(&adapter, surface_width, surface_height)
            .ok_or(RendererInitError::SurfaceUnsupported)?;
        surface.configure(&device, &surface_config);

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("corewar-viz-uniforms"),
            contents: bytemuck::bytes_of(&RendererUniforms::from_config(&config)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let cell_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("corewar-viz-cells"),
            contents: bytemuck::cast_slice(&core_state.as_gpu_cells()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("corewar-viz-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("corewar-viz-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: cell_buffer.as_entire_binding(),
                },
            ],
        });

        let vertex_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("corewar-viz-grid-vertex"),
            source: wgpu::ShaderSource::Wgsl(GRID_VERTEX_SHADER.into()),
        });
        let fragment_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("corewar-viz-grid-fragment"),
            source: wgpu::ShaderSource::Wgsl(GRID_FRAGMENT_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("corewar-viz-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("corewar-viz-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &fragment_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });

        Ok(Self {
            config,
            core_state,
            instance: Some(instance),
            surface: Some(surface),
            device: Some(device),
            queue: Some(queue),
            surface_config: Some(surface_config),
            pipeline: Some(pipeline),
            bind_group_layout: Some(bind_group_layout),
            bind_group: Some(bind_group),
            uniform_buffer: Some(uniform_buffer),
            cell_buffer: Some(cell_buffer),
        })
    }

    pub fn update_cells(&mut self, events: &[CycleEvent]) {
        self.core_state.apply_events(events);
        self.upload_core_state();
    }

    pub fn set_core_state(&mut self, core_state: CoreState) {
        self.core_state = core_state;
        self.upload_core_state();
    }

    fn upload_core_state(&self) {
        if let (Some(queue), Some(cell_buffer)) = (self.queue.as_ref(), self.cell_buffer.as_ref()) {
            let gpu_cells = self.core_state.as_gpu_cells();
            queue.write_buffer(cell_buffer, 0, bytemuck::cast_slice(&gpu_cells));
        }
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        let (surface, device, surface_config) = match (
            self.surface.as_ref(),
            self.device.as_ref(),
            self.surface_config.as_mut(),
        ) {
            (Some(surface), Some(device), Some(surface_config)) => {
                (surface, device, surface_config)
            }
            _ => return,
        };

        surface_config.width = width.max(1);
        surface_config.height = height.max(1);
        surface.configure(device, surface_config);
    }

    pub fn render(&mut self) {
        let (surface, device, queue, surface_config, pipeline, bind_group) = match (
            self.surface.as_ref(),
            self.device.as_ref(),
            self.queue.as_ref(),
            self.surface_config.as_ref(),
            self.pipeline.as_ref(),
            self.bind_group.as_ref(),
        ) {
            (
                Some(surface),
                Some(device),
                Some(queue),
                Some(surface_config),
                Some(pipeline),
                Some(bind_group),
            ) => (surface, device, queue, surface_config, pipeline, bind_group),
            _ => return,
        };

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.configure(device, surface_config);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                error!("surface acquisition failed: out of memory");
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => {
                warn!("surface acquisition timed out");
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("corewar-viz-render-encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("corewar-viz-render-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.config.background_color[0] as f64,
                            g: self.config.background_color[1] as f64,
                            b: self.config.background_color[2] as f64,
                            a: self.config.background_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..6, 0..1);
        }

        queue.submit(Some(encoder.finish()));
        frame.present();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_state_updates_heat_and_cycle() {
        let mut core = CoreState::new(4, 4);
        core.apply_events(&[CycleEvent::Write {
            address: 3,
            warrior_id: 7,
        }]);

        assert_eq!(core.current_cycle, 1);
        assert_eq!(core.cells[3].owner, Some(7));
        assert_eq!(core.cells[3].heat, 1.0);
    }
}
