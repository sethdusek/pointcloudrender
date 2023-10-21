use image::{ImageBuffer, Luma, Rgba};
use nalgebra::{Matrix4, Point3};
use wgpu::util::DeviceExt;

use crate::view_params::ViewParams;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// TODO: refactor into view_params.rs
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ViewUniform {
    view: [f32; 16],
}
impl From<ViewParams> for ViewUniform {
    fn from(view_params: ViewParams) -> ViewUniform {
        let matrix = OPENGL_TO_WGPU_MATRIX * view_params.projection * view_params.camera;
        ViewUniform {
            view: matrix.as_slice().try_into().unwrap(),
        }
    }
}

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    vertex_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
    pub window: winit::window::Window,
    pub view_params: ViewParams,
}

impl Renderer {
    pub async fn new(
        window: winit::window::Window,
        image: ImageBuffer<Rgba<u8>, Vec<u8>>,
        depth: ImageBuffer<Luma<u8>, Vec<u8>>,
    ) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::POLYGON_MODE_POINT,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb()) // TODO: to srgb or not to srgb?
            .unwrap_or(surface_caps.formats[0]);

        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|&mode| mode == wgpu::PresentMode::Immediate)
            .unwrap_or(surface_caps.present_modes[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            // I'm guessing we need TEXTURE_BINDING to use in a compute shader
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: dbg!(surface_caps.alpha_modes[0]),
            // TODO: probably should request an RGBA image
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);



        let vertex_buffer = Renderer::load_image(&device, image, depth);
        let (view_params, camera_buffer) = Renderer::create_camera_buffer(&device);
        let now = std::time::Instant::now();
        let (camera_bind_group, render_pipeline ) = Renderer::create_pipeline(&device, &camera_buffer, &surface_config);
        println!(
            "Time to create render pipeline: {:?}",
            std::time::Instant::now() - now
        );
        Renderer {
            surface,
            device,
            queue,
            surface_config,
            size,
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
            view_params,
            render_pipeline,
            window,
        }
    }

    fn load_image(
        device: &wgpu::Device,
        image: ImageBuffer<Rgba<u8>, Vec<u8>>,
        depth: ImageBuffer<Luma<u8>, Vec<u8>>,
    ) -> wgpu::Buffer {
        let dims = image.dimensions();
        assert_eq!(image.dimensions(), depth.dimensions());
        let mut vertices = Vec::with_capacity((dims.0 * dims.1) as usize);
        let min_depth = depth.rows().flatten().map(|luma| luma.0[0]).min().unwrap();
        let max_depth =
            (depth.rows().flatten().map(|luma| luma.0[0]).max().unwrap() - min_depth) as f32;
        // Generate vertices for each pixel. OpenGL coordinates have a minimum of -1 and maximum of 1
        for (y, (r1, r2)) in image.rows().zip(depth.rows()).enumerate() {
            for (x, (c1, c2)) in r1.zip(r2).enumerate() {
                vertices.push(Vertex {
                    // TODO: wgpu uses -1 to 1.0 for x/y and 0.0 to 1.0 for z. For now I'll map this to wgpu coordinates but revert later to opengl-to-wgpu conversion
                    position: [
                        (x as f32 / dims.0 as f32) * 2.0 - 1.0,
                        // Top of the screen is +1 in OpenGL
                        (y as f32 / dims.1 as f32) * -2.0 + 1.0,
                        ((c2.0[0] - min_depth) as f32 / (max_depth - min_depth as f32)) * -2.0 + 0.9,
                    ],
                    color: [
                        c1.0[0] as f32 / 255.0,
                        c1.0[1] as f32 / 255.0,
                        c1.0[2] as f32 / 255.0,
                        0.0,
                    ],
                });
            }
        }
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        vertex_buffer
    }

    fn create_camera_buffer(device: &wgpu::Device) -> (ViewParams, wgpu::Buffer) {
        let eye = Point3::new(0.0f32, 0.0, 1.0);
        let look_at = Point3::new(0.0, 0.0, -0.1);
        let view_params = ViewParams::new(
            eye,
            look_at,
            Matrix4::new_orthographic(-1.0f32, 1.0, -1.0, 1.0, 0.0, 3.0),
        );
        let view_uniform = ViewUniform::from(view_params);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera"),
            contents: bytemuck::cast_slice(&[view_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        (view_params, camera_buffer)
    }

    // Create render pipeline including all bindgroups
    fn create_pipeline(
        device: &wgpu::Device,
        camera_buffer: &wgpu::Buffer,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> (wgpu::BindGroup, wgpu::RenderPipeline) {
        let raster_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/raster.wgsl"));

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bindgroup Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bindgroup"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Raster Pipeline"),
            vertex: wgpu::VertexState {
                module: &raster_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &raster_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            layout: Some(&render_pipeline_layout),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,  // doesn't matter
                cull_mode: Some(wgpu::Face::Back), // doesn't matter also
                polygon_mode: wgpu::PolygonMode::Fill, // doesn't matter..?
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        (camera_bind_group, render_pipeline)
    }

    pub fn update_camera(&mut self) {
        let view_uniform = ViewUniform::from(self.view_params);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[view_uniform]));
    }
    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Point Cloud Render Encoder"),
                });
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.draw(
                0..(self.vertex_buffer.size() as usize / std::mem::size_of::<Vertex>()) as u32,
                0..1,
            )
        }
        self.queue.submit(std::iter::once(command_encoder.finish()));
        output.present();
        Ok(())
    }
}
