use wgpu::util::DeviceExt;

use crate::texture::Texture;
pub struct BackgroundShader {
    pub textures: [(Texture, Texture); 2],
    convergence_tracker: wgpu::Buffer,
    // TODO: 2 bindgroups
    bindgroups: [wgpu::BindGroup; 2],
    compute_pipeline: wgpu::ComputePipeline,
}

impl BackgroundShader {
    pub fn new(device: &wgpu::Device, dims: (u32, u32)) -> Self {
        // TODO: can't bind depth textures to compute shaders. also change flags for compute
        let textures = [
            (
                Texture::new(
                    device,
                    dims,
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::STORAGE_BINDING,
                    "bs_tex_01",
                ),
                Texture::new(
                    device,
                    dims,
                    crate::wgpu_renderer::DEPTH_STORAGE_FORMAT,
                    wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::STORAGE_BINDING,
                    "bs_depth_tex_01",
                ),
            ),
            (
                Texture::new(
                    device,
                    dims,
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::STORAGE_BINDING,
                    "bs_tex_02",
                ),
                Texture::new(
                    device,
                    dims,
                    crate::wgpu_renderer::DEPTH_STORAGE_FORMAT,
                    wgpu::TextureUsages::COPY_SRC
                        | wgpu::TextureUsages::COPY_DST
                        | wgpu::TextureUsages::STORAGE_BINDING,
                    "bs_depth_tex_02",
                ),
            ),
        ];

        let convergence_tracker = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: &[0u8],
            usage: wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::MAP_WRITE
                | wgpu::BufferUsages::UNIFORM,
        });
        let (bindgroups, compute_pipeline) =
            BackgroundShader::create_compute_pipeline(&device, &textures, &convergence_tracker);
        BackgroundShader {
            textures,
            convergence_tracker,
            bindgroups,
            compute_pipeline,
        }
    }

    pub fn create_compute_pipeline(
        device: &wgpu::Device,
        textures: &[(Texture, Texture); 2],
        convergence_tracker: &wgpu::Buffer,
    ) -> ([wgpu::BindGroup; 2], wgpu::ComputePipeline) {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CS Bindgroup Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadOnly,
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(1.try_into().unwrap()),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadOnly,
                        format: crate::wgpu_renderer::DEPTH_STORAGE_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: crate::wgpu_renderer::DEPTH_STORAGE_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let create_bind_group =
            |color0: &Texture, depth0: &Texture, color1: &Texture, depth1: &Texture| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("CS Bindgroup"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&color0.texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&color1.texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Buffer(
                                convergence_tracker.as_entire_buffer_binding(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(&depth0.texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(&depth1.texture_view),
                        },
                    ],
                })
            };
        dbg!(textures[0].0.texture.format());
        let bind_groups = [
            create_bind_group(
                &textures[0].0,
                &textures[0].1,
                &textures[1].0,
                &textures[1].1,
            ),
            create_bind_group(
                &textures[1].0,
                &textures[1].1,
                &textures[0].0,
                &textures[0].1,
            ),
        ];

        let compute_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/background_shader.wgsl"));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("background_filling_layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("background_filling"),
            module: &compute_shader,
            layout: Some(&layout),
            entry_point: "main",
        });
        (bind_groups, compute_pipeline)
    }

    pub fn run(
        &self,
        device: &wgpu::Device,
        command_encoder: &mut wgpu::CommandEncoder,
        initial_texture: &Texture,
        initial_depth: &Texture,
        iters: usize,
    ) {
        let dims = (
            initial_texture.texture.width(),
            initial_texture.texture.height(),
        );
        command_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &initial_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &self.textures[0].0.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: dims.0,
                height: dims.1,
                depth_or_array_layers: 1,
            },
        );
        command_encoder.copy_texture_to_texture(
            wgpu::ImageCopyTexture {
                texture: &initial_depth.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTexture {
                texture: &self.textures[0].1.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: dims.0,
                height: dims.1,
                depth_or_array_layers: 1,
            },
        );
        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("bf_compute_pass"),
                    ..Default::default()
                });
            compute_pass.set_pipeline(&self.compute_pipeline);
            for iter in 0..iters {
                compute_pass.set_bind_group(0, &self.bindgroups[iter % 2], &[]);
                compute_pass.dispatch_workgroups((dims.0 + 7) / 8, (dims.1 + 7) / 8, 1);
            }
        }
    }

    pub fn count(&self) -> u32 {
        todo!()
    }
}
