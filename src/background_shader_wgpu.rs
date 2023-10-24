use crate::texture::Texture;
pub struct BackgroundShader {
    pub textures: [(Texture, Texture); 2],
    // TODO: 2 bindgroups
    bindgroup: wgpu::BindGroup,
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
                    wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::STORAGE_BINDING,
                    "bs_tex_01",
                ),
                Texture::new(
                    device,
                    dims,
                    crate::wgpu_renderer::DEPTH_FORMAT,
                    wgpu::TextureUsages::COPY_SRC,
                    "bs_depth_tex_01",
                ),
            ),
            (
                Texture::new(
                    device,
                    dims,
                    wgpu::TextureFormat::Bgra8Unorm,
                    wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
                    "bs_tex_02",
                ),
                Texture::new(
                    device,
                    dims,
                    crate::wgpu_renderer::DEPTH_FORMAT,
                    wgpu::TextureUsages::COPY_SRC,
                    "bs_depth_tex_02",
                ),
            ),
        ];
        let (bindgroup, compute_pipeline) =
            BackgroundShader::create_compute_pipeline(&device, &textures);
        BackgroundShader {
            textures,
            bindgroup,
            compute_pipeline,
        }
    }

    pub fn create_compute_pipeline(
        device: &wgpu::Device,
        textures: &[(Texture, Texture); 2],
    ) -> (wgpu::BindGroup, wgpu::ComputePipeline) {
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
            ],
        });
        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CS Bindgroup"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&textures[0].0.texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&textures[1].0.texture_view),
                },
            ],
        });

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
        (compute_bind_group, compute_pipeline)
    }

    pub fn run(&self, device: &wgpu::Device, queue: &wgpu::Queue, initial_texture: &Texture) {
        let dims = (
            initial_texture.texture.width(),
            initial_texture.texture.height(),
        );
        let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command_encoder"),
        });
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
                depth_or_array_layers: 0,
            },
        );
        {
            let mut compute_pass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("bf_compute_pass"),
                });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.bindgroup, &[]);
            compute_pass.dispatch_workgroups(dims.0, dims.1, 1);
        }
        let command_buffer = command_encoder.finish();
        queue.submit(std::iter::once(command_buffer));
    }
}
