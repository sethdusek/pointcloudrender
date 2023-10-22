/// Ported over from learn wgpu tutorial
pub struct Texture {
    pub texture: wgpu::Texture,
        // TODO: do we need sampler/view if we're not using the texture in a fragment shader? Update: don't need sampler for depth buffer, but need view
    pub texture_view: wgpu::TextureView
}

impl Texture {
    pub fn new_depth(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, format: wgpu::TextureFormat) -> Texture {
        let size = wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth buffer"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[] // TODO?
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Texture {
            texture,
            texture_view
        }
    }
}
