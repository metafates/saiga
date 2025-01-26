use std::{borrow::Cow, mem};

use wgpu::util::DeviceExt;

use crate::display::context;

use super::math;

const MAX_INSTANCES: usize = 5_000;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
struct Uniform {
    transform: [f32; 16],
    scale: f32,
    padding: [f32; 3],
}

impl Uniform {
    fn new(transformation: [f32; 16], scale: f32) -> Uniform {
        Self {
            transform: transformation,
            scale,
            // Ref: https://github.com/iced-rs/iced/blob/bc62013b6cde52174bf4c4286939cf170bfa7760/wgpu/src/quad.rs#LL295C6-L296C68
            // Uniforms must be aligned to their largest member,
            // this uses a mat4x4<f32> which aligns to 16, so align to that
            padding: [0.0; 3],
        }
    }
}

impl Default for Uniform {
    fn default() -> Self {
        #[rustfmt::skip]
        const IDENTITY_MATRIX: [f32; 16] = [
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ];

        Self {
            transform: IDENTITY_MATRIX,
            scale: 1.0,
            padding: [0.0; 3],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    pub fn new(position: [f32; 2]) -> Self {
        Self { position }
    }
}

const QUAD_INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];

#[derive(Debug, PartialEq, Default, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod)]
#[repr(C)]
pub struct Rect {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub size: [f32; 2],
}

#[derive(Debug)]
pub struct Brush {
    uniform_buf: wgpu::Buffer,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instances_buf: wgpu::Buffer,

    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,

    index_count: usize,
    current_transform: [f32; 16],
}

impl Brush {
    pub fn new(ctx: &context::Context) -> Self {
        let device = &ctx.device;

        let rect_vertices = vec![
            Vertex::new([0.0, 0.0]),
            Vertex::new([0.5, 0.0]),
            Vertex::new([0.5, 1.0]),
            Vertex::new([0.0, 1.0]),
        ];

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: mem::size_of::<Uniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&rect_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&QUAD_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        mem::size_of::<Uniform>() as wgpu::BufferAddress
                    ),
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buf,
                    offset: 0,
                    size: None,
                }),
            }],
            label: Some("rect::Pipeline uniforms"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let vertex_buffers = [
            wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<Rect>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array!(
                    1 => Float32x2,
                    2 => Float32x4,
                    3 => Float32x2,
                ),
            },
        ];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Cw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let instances_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instances Buffer"),
            size: mem::size_of::<Rect>() as u64 * MAX_INSTANCES as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            uniform_buf,
            vertex_buf,
            index_buf,
            instances_buf,
            bind_group,
            pipeline,
            index_count: QUAD_INDICES.len(),
            current_transform: [0.0; 16],
        }
    }

    pub fn resize(&mut self, ctx: &mut context::Context) {
        let transform: [f32; 16] =
            math::orthographic_projection(ctx.size.width as f32, ctx.size.height as f32);

        let queue = &mut ctx.queue;

        if transform != self.current_transform {
            let uniform = Uniform::new(transform, ctx.size.scale_factor as f32);

            queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniform));

            self.current_transform = transform;
        }
    }

    pub fn draw(
        &mut self,
        ctx: &mut context::Context,
        rpass: &mut wgpu::RenderPass,
        rects: Vec<Rect>,
    ) {
        if rects.is_empty() {
            return;
        }

        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        rpass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        rpass.set_vertex_buffer(1, self.instances_buf.slice(..));

        for batch in rects.chunks(MAX_INSTANCES) {
            let instance_bytes = bytemuck::cast_slice(batch);

            ctx.queue
                .write_buffer(&self.instances_buf, 0, instance_bytes);
            rpass.draw_indexed(0..self.index_count as u32, 0, 0..batch.len() as u32);
        }
    }
}
