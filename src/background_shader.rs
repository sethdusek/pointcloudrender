use std::rc::Rc;

use glium::{
    buffer::Buffer, framebuffer::SimpleFrameBuffer, program::ComputeShader,
    texture::DepthTexture2d, uniforms::MagnifySamplerFilter, Display, Surface, Texture2d, uniform,
};

pub struct BackgroundShader {
    display: Rc<Display>,
    shader: ComputeShader,
    buffers: [(Texture2d, DepthTexture2d); 2],
    // A simple boolean that lets us know no background filling was done in this iteration
    converged_tracker: Buffer<u32>,
}

impl BackgroundShader {
    pub fn new(display: Rc<Display>, dims: (u32, u32)) -> Result<Self, Box<dyn std::error::Error>> {
        let shader = ComputeShader::from_source(&*display, include_str!("background_shader.glsl"))?;
        let buffers = [
            (
                Texture2d::empty(&*display, dims.0, dims.1)?,
                DepthTexture2d::empty(&*display, dims.0, dims.1)?,
            ),
            (
                Texture2d::empty(&*display, dims.0, dims.1)?,
                DepthTexture2d::empty(&*display, dims.0, dims.1)?,
            ),
        ];
        //
        let converged_tracker = Buffer::new(
            &*display,
            &0u32,
            glium::buffer::BufferType::AtomicCounterBuffer,
            glium::buffer::BufferMode::Persistent,
        )?;

        Ok(Self {
            display,
            shader,
            buffers,
            converged_tracker,
        })
    }

    // Run 1 iteration of the background filling process
    fn iterate(&mut self) -> bool {
        // TODO
        false
    }
    pub fn run(
        &mut self,
        initial_texture: &Texture2d,
        initial_depth: &DepthTexture2d,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let simple_buffer =
            SimpleFrameBuffer::with_depth_buffer(&*self.display, initial_texture, initial_depth)?;
        let target = SimpleFrameBuffer::with_depth_buffer(
            &*self.display,
            &self.buffers[0].0,
            &self.buffers[0].1,
        )?;
        simple_buffer.fill(&target, MagnifySamplerFilter::Linear);

        let in_unit = self.buffers[0]
            .0
            .image_unit(glium::uniforms::ImageUnitFormat::RGBA8UI)?
        .set_access(glium::uniforms::ImageUnitAccess::Read);
        let out_unit = self.buffers[1]
            .0
            .image_unit(glium::uniforms::ImageUnitFormat::RGBA8UI)?
        .set_access(glium::uniforms::ImageUnitAccess::Write);


        let dims = (in_unit.0.width(), in_unit.0.height());
        let uniforms = uniform! {
            uWidth: dims.0,
            uHeight: dims.1,
            in_image: in_unit,
            out_image: out_unit,
            converged: &self.converged_tracker

        };
        self.shader.execute(uniforms, dims.0, dims.1, 1);
        Ok(())
    }
    // Return the buffer that was last filled in. Calling this before BackgroundShader::run will probably result in garbage
    pub fn front_buffer(&self) -> &(Texture2d, DepthTexture2d) {
        &self.buffers[1]
    }
    pub fn count(&self) -> u32 {
        self.converged_tracker.read().unwrap()
    }

}
