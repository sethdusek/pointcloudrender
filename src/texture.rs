/// Ported over from learn wgpu tutorial
pub struct Texture {
    pub texture: wgpu::Texture,
    // TODO: do we need sampler/view if we're not using the texture in a fragment shader? Update: don't need sampler for depth buffer, but need view
    pub texture_view: wgpu::TextureView,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        (width, height): (u32, u32),
        format: wgpu::TextureFormat,
        texture_usage: wgpu::TextureUsages,
        label: &str,
    ) -> Texture {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: texture_usage,
            view_formats: &[], // TODO?
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Texture {
            texture,
            texture_view,
        }
    }
}
