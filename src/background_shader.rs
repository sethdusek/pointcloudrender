use std::rc::Rc;

use glium::{
    buffer::Buffer, framebuffer::SimpleFrameBuffer, glutin::surface::WindowSurface,
    program::ComputeShader, texture::DepthTexture2d, uniform, uniforms::MagnifySamplerFilter,
    Display, Surface, Texture2d,
};

pub struct BackgroundShader {
    display: Rc<Display<WindowSurface>>,
    shader: ComputeShader,
    buffers: [(Texture2d, Texture2d); 2],
    // A simple boolean that lets us know no background filling was done in this iteration
    converged_tracker: Buffer<u32>,
}

impl BackgroundShader {
    pub fn new(
        display: Rc<Display<WindowSurface>>,
        dims: (u32, u32),
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let shader = ComputeShader::from_source(&*display, include_str!("background_shader.glsl"))?;
        let buffers = [
            (
                Texture2d::empty_with_format(
                    &*display,
                    glium::texture::UncompressedFloatFormat::U8U8U8U8,
                    glium::texture::MipmapsOption::NoMipmap,
                    dims.0,
                    dims.1,
                )?,
                Texture2d::empty_with_format(
                    &*display,
                    glium::texture::UncompressedFloatFormat::F32,
                    glium::texture::MipmapsOption::NoMipmap,
                    dims.0,
                    dims.1,
                )?,
            ),
            (
                Texture2d::empty_with_format(
                    &*display,
                    glium::texture::UncompressedFloatFormat::U8U8U8U8,
                    glium::texture::MipmapsOption::NoMipmap,
                    dims.0,
                    dims.1,
                )?,
                Texture2d::empty_with_format(
                    &*display,
                    glium::texture::UncompressedFloatFormat::F32,
                    glium::texture::MipmapsOption::NoMipmap,
                    dims.0,
                    dims.1,
                )?,
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
    fn iterate(&mut self) {
        self.converged_tracker.write(&0);

        let in_unit_color = self.buffers[0]
            .0
            .image_unit(glium::uniforms::ImageUnitFormat::RGBA8)
            .unwrap()
            .set_access(glium::uniforms::ImageUnitAccess::Read);
        let in_unit_depth = self.buffers[0]
            .1
            .image_unit(glium::uniforms::ImageUnitFormat::R32F)
            .unwrap()
            .set_access(glium::uniforms::ImageUnitAccess::Read);

        let out_unit_color = self.buffers[1]
            .0
            .image_unit(glium::uniforms::ImageUnitFormat::RGBA8)
            .unwrap()
            .set_access(glium::uniforms::ImageUnitAccess::Write);
        let out_unit_depth = self.buffers[1]
            .1
            .image_unit(glium::uniforms::ImageUnitFormat::R32F)
            .unwrap()
            .set_access(glium::uniforms::ImageUnitAccess::Write);

        let dims = (in_unit_color.0.width(), in_unit_color.0.height());
        let uniforms = uniform! {
            input_image: in_unit_color,
            input_depth: in_unit_depth,
            output_image: out_unit_color,
            output_depth: out_unit_depth,
            uWidth: dims.0,
            uHeight: dims.1,
            converged: &self.converged_tracker
        };

        //dbg!(std::time::Instant::now() - start);
        self.shader.execute(
            uniforms,
            dims.0 / 8 + dims.0 % 8,
            dims.1 / 8 + dims.1 % 8,
            1,
        );
        // self.shader.execute(
        //     uniforms,
        //     dims.0,
        //     dims.1,
        //     1,
        // );
        //self.count() == 0
    }
    pub fn run(
        &mut self,
        initial_texture: &Texture2d,
        initial_depth: &Texture2d,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let simple_buffer_color = SimpleFrameBuffer::new(&*self.display, initial_texture)?;
        let target_color = SimpleFrameBuffer::new(&*self.display, &self.buffers[0].0)?;
        simple_buffer_color.fill(&target_color, MagnifySamplerFilter::Linear);
        let simple_buffer_depth = SimpleFrameBuffer::new(&*self.display, initial_depth)?;
        let target_depth = SimpleFrameBuffer::new(&*self.display, &self.buffers[0].1)?;
        simple_buffer_depth.fill(&target_depth, MagnifySamplerFilter::Linear);

        self.buffers[0].0.sync_shader_writes_for_surface();
        self.buffers[0].1.sync_shader_writes_for_surface();
        let mut iters = 0;
        let mut start = std::time::Instant::now();

        let mut sum = 0;

        while iters < 40 {
            let now = std::time::Instant::now();
            self.iterate();
            self.buffers.swap(0, 1);
            // println!(
            //     "{iters} iteration of background shading took {}us",
            //     (now - start).as_micros()
            // );
            sum+=(now - start).as_micros();
            if iters % 10 == 0 && self.count() == 0 {
                break;
            }
            start = now;
            iters += 1;
        }
        println!("Average: {}", sum.checked_div(iters).unwrap_or(0));
        Ok(())
    }
    // Return the buffer that was last filled in. Calling this before BackgroundShader::run will probably result in garbage
    pub fn front_buffer(&self) -> &(Texture2d, Texture2d) {
        // Change back to 1
        &self.buffers[1]
    }
    pub fn count(&self) -> u32 {
        self.converged_tracker.read().unwrap()
    }

    pub fn set_shader(&mut self, src: &str) {
        let shader = ComputeShader::from_source(&*self.display, src).unwrap();
        self.shader = shader;
    }
}
