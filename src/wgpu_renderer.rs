use image::{ImageBuffer, Luma, Rgba};
use nalgebra::{Matrix4, Point3};
use wgpu::{include_wgsl, util::DeviceExt};

use crate::{filling_shader_wgpu::FillingShader, texture::Texture, view_params::ViewParams};

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

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
// What gets used in the depth texture used for compute shading
pub const DEPTH_STORAGE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

// Describes state of window (and surface)
pub struct HeadState {
    pub window: winit::window::Window,
    pub surface: wgpu::Surface,
    surface_config: wgpu::SurfaceConfiguration,
}

impl HeadState {
    fn from_surface(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        window: winit::window::Window,
        surface: wgpu::Surface,
    ) -> Self {
        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !dbg!(f).is_srgb()) // TODO: to srgb or not to srgb?
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
                | wgpu::TextureUsages::COPY_DST
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

        dbg!(surface_format);

        HeadState {
            window,
            surface,
            surface_config,
        }
    }
}

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    vertex_buffer: wgpu::Buffer,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    target_texture: Texture,
    // Target depth is what we render depth to and use in any compute shaders. Depth_texture is what's used for depth testing
    target_depth: Texture,
    depth_texture: Texture,
    render_pipeline: wgpu::RenderPipeline,
    background_shader: Option<FillingShader>,
    occlusion_shader: Option<FillingShader>,
    pub view_params: ViewParams,
    pub head_state: Option<HeadState>,
    pub background_shading_iters: u32,
    pub occlusion_shading_iters: u32,
}

impl Renderer {
    pub async fn new(
        window: Option<winit::window::Window>,
        image: ImageBuffer<Rgba<u8>, Vec<u8>>,
        depth: ImageBuffer<Luma<u8>, Vec<u8>>,
        background_filling: bool,
        occlusion_filling: bool,
    ) -> Self {
        let size = image.dimensions();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            dx12_shader_compiler: Default::default(),
            flags: wgpu::InstanceFlags::from_build_config(), // bless wgpu for adding this feature
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });

        let surface = window
            .as_ref()
            .map(|window| unsafe { instance.create_surface(&window).unwrap() });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::POLYGON_MODE_POINT
                        | wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | wgpu::Features::BGRA8UNORM_STORAGE
                        | wgpu::Features::MAPPABLE_PRIMARY_BUFFERS,
                    limits: wgpu::Limits::default(),
                    label: None,
                },
                None,
            )
            .await
            .unwrap();

        let head_state = if let Some(window) = window {
            Some(HeadState::from_surface(
                &device,
                &adapter,
                window,
                surface.unwrap(),
            ))
        } else {
            None
        };
        let texture_format = head_state
            .as_ref()
            .map(|hs| hs.surface_config.format)
            .unwrap_or(wgpu::TextureFormat::Bgra8Unorm);
        // Generate buffers and other on-device resources
        let vertex_buffer = Renderer::load_image(&device, image, depth);
        let (view_params, camera_buffer) = Renderer::create_camera_buffer(&device);
        let now = std::time::Instant::now();
        let (camera_bind_group, render_pipeline) = Renderer::create_pipeline(
            &device,
            &camera_buffer,
            texture_format,
            DEPTH_STORAGE_FORMAT,
        );
        eprintln!(
            "Time to create render pipeline: {:?}",
            std::time::Instant::now() - now
        );

        let target_texture = Texture::new(
            &device,
            size,
            texture_format,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            "Render Buffer",
        );
        let target_depth = Texture::new(
            &device,
            size,
            DEPTH_STORAGE_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            "Depth Render Buffer",
        );

        let depth_texture = Texture::new(
            &device,
            size,
            DEPTH_FORMAT,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
            "Depth Buffer",
        );

        let background_shader = if background_filling {
            Some(FillingShader::new(
                &device,
                size,
                wgpu::include_wgsl!("shaders/background_shader.wgsl"),
            ))
        } else {
            None
        };
        let occlusion_shader = if occlusion_filling {
            Some(FillingShader::new(
                &device,
                size,
                wgpu::include_wgsl!("shaders/occlusion_shader.wgsl"),
            ))
        } else {
            None
        };

        Renderer {
            device,
            queue,
            vertex_buffer,
            camera_buffer,
            camera_bind_group,
            target_texture,
            target_depth,
            depth_texture,
            view_params,
            render_pipeline,
            background_shader,
            occlusion_shader,
            head_state,
            background_shading_iters: 5,
            occlusion_shading_iters: 1,
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
                    position: [
                        (x as f32 / dims.0 as f32) * 2.0 - 1.0,
                        // Top of the screen is +1 in OpenGL
                        (y as f32 / dims.1 as f32) * -2.0 + 1.0,
                        ((c2.0[0] - min_depth) as f32 / (max_depth - min_depth as f32)) * -2.0
                            + 0.9,
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
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
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
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: depth_format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[view_uniform]),
        );
    }
    pub fn render(
        &mut self,
        background_filling_toggle: bool,
        occlusion_filling_toggle: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = self
            .head_state
            .as_ref()
            .map(|hs| hs.surface.get_current_texture())
            .transpose()?;
        let view = &self.target_texture.texture_view;
        let depth_view = &self.target_depth.texture_view;
        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Point Cloud Render Encoder"),
                });
        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &depth_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 0.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                    view: &self.depth_texture.texture_view,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.draw(
                0..(self.vertex_buffer.size() as usize / std::mem::size_of::<Vertex>()) as u32,
                0..1,
            );
        }
        let mut output_texture = &self.target_texture;
        let mut output_depth = &self.target_depth;

        if let Some(background_shader) = &self.background_shader {
            if background_filling_toggle {
                background_shader.run(
                    &mut command_encoder,
                    output_texture,
                    output_depth,
                    self.background_shading_iters,
                );
                output_texture = &background_shader.textures[1].0;
                output_depth = &background_shader.textures[1].1;
            }
        }

        if let Some(occlusion_shader) = &self.occlusion_shader {
            if occlusion_filling_toggle {
                occlusion_shader.run(
                    &mut command_encoder,
                    &output_texture,
                    &output_depth,
                    self.occlusion_shading_iters,
                );
                output_texture = &occlusion_shader.textures[1].0;
                output_depth = &occlusion_shader.textures[1].1;
            }
        }

        let dst = if let Some(output) = &output {
            &output.texture
        } else {
            &self.target_texture.texture
        };

        command_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &output_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: dst,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: dst.width(),
                height: dst.height(),
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(std::iter::once(command_encoder.finish()));

        // TODO: clean up this mess
        if let Some(output) = output {
            output.present();
        }
        Ok(())
    }

    fn read_texture(&self, texture: &wgpu::Texture) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let bpp = texture.format().block_size(None).unwrap();
        // let bpp = match texture.format() {
        //     wgpu::TextureFormat::
        // }
        let row_bytes = (bpp * texture.width()) as u64;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u64;
        let padding = (alignment - row_bytes % alignment) % alignment;
        let padded_row_bytes = row_bytes + padding;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CPU Buffer"),
            size: padded_row_bytes * texture.height() as u64,
            mapped_at_creation: false,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        });

        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("screenshot_encoder"),
                });
        let size = wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        };
        command_encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some((padded_row_bytes as u32).try_into()?),
                    rows_per_image: None,
                },
            },
            size,
        );
        self.queue.submit(std::iter::once(command_encoder.finish()));
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.unwrap());
        self.device.poll(wgpu::Maintain::Wait);

        let mut output_buffer: Vec<u8> =
            Vec::with_capacity((row_bytes * texture.height() as u64) as usize);
        let slice_view = slice.get_mapped_range();
        for chunk in slice_view.chunks(padded_row_bytes as usize) {
            output_buffer.extend(&chunk[..row_bytes as usize]);
        }
        std::mem::drop(slice_view);

        Ok(output_buffer)
    }
    pub fn read_front_buffer(
        &self,
    ) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn std::error::Error>> {
        let mut buf = self.read_texture(&self.target_texture.texture)?;
        // Convert bgra to rgba
        if let wgpu::TextureFormat::Bgra8Unorm = self.target_texture.texture.format() {
            for chunk in buf.chunks_mut(4) {
                chunk.swap(0, 2);
            }
        }
        let image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_vec(
            self.target_texture.texture.width(),
            self.target_texture.texture.height(),
            buf,
        )
        .unwrap();
        Ok(image)
    }
    pub fn save_screenshot(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let image = self.read_front_buffer()?;
        image.save(path)?;
        Ok(())
    }
}
