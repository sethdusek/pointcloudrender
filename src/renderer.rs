use std::{rc::Rc, borrow::Cow};

use glium::{

    framebuffer::{MultiOutputFrameBuffer, SimpleFrameBuffer},
    glutin::surface::WindowSurface,
    implement_vertex,
    texture::{DepthTexture2d, RawImage2d},
    uniform, Display, DrawParameters, Program, Surface, Texture2d, VertexBuffer,
};
use image::{ImageBuffer, Luma, Rgba};
use nalgebra::{Matrix4, Point3};


use crate::{view_params::ViewParams, background_shader::BackgroundShader};


#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}
implement_vertex!(Vertex, position, color);

pub struct Renderer {
    display: Rc<Display<WindowSurface>>,
    program: Program,
    vertex_buffer: VertexBuffer<Vertex>,
    target_texture: Texture2d,
    target_depth: Texture2d,
    pub view_params: ViewParams,
    background_shader: Option<BackgroundShader>,
    raster: bool,
}

impl Renderer {
    pub fn new(
        display: Display<WindowSurface>,
        image: ImageBuffer<Rgba<u8>, Vec<u8>>,
        depth: ImageBuffer<Luma<u8>, Vec<u8>>,
        background_filling: bool,
        raster: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        assert_eq!(image.dimensions(), depth.dimensions());
        let dims = image.dimensions();
        let program = Program::from_source(
            &display,
            include_str!("shaders/vertex.glsl"),
            include_str!("shaders/fragment.glsl"),
            None,
        )?;
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
        println!(
            "Min depth: {:?}",
            vertices
                .iter()
                .map(|v| float_ord::FloatOrd(v.position[2]))
                .min()
                .unwrap()
        );
        println!(
            "Max depth: {:?}",
            vertices
                .iter()
                .map(|v| float_ord::FloatOrd(v.position[2]))
                .max()
                .unwrap()
        );
        let vertex_buffer = VertexBuffer::new(&display, &vertices)?;

        let eye = Point3::new(0.0f32, 0.0, 1.0);
        let look_at = Point3::new(0.0, 0.0, -0.1);

        // TODO: figure out projection. This is just a placeholder
        let view_params = ViewParams::new(
            eye,
            look_at,
            Matrix4::new_orthographic(-1.0f32, 1.0, -1.0, 1.0, 0.0, 3.0),
        );
        // println!("Min depth camera: {:?}", vertices.iter().map(|v| view_params.camera * Vector4::new(v.position[0], v.position[1], v.position[2], 1.0)).
        //                                                 map(|v| float_ord::FloatOrd(v[2])).min().unwrap());
        // println!("Max depth camera: {:?}", vertices.iter().map(|v| view_params.camera * Vector4::new(v.position[0], v.position[1], v.position[2], 1.0)).
        //                                                 map(|v| float_ord::FloatOrd(v[2])).max().unwrap());
        // println!("Min depth projection: {:?}", vertices.iter().map(|v| view_params.projection * view_params.camera * Vector4::new(v.position[0], v.position[1], v.position[2], 1.0)).
        //                                                 map(|v| float_ord::FloatOrd(v[2])).min().unwrap());
        // println!("Max depth projection: {:?}", vertices.iter().map(|v| view_params.projection * view_params.camera * Vector4::new(v.position[0], v.position[1], v.position[2], 1.0)).
        //                                                 map(|v| float_ord::FloatOrd(v[2])).max().unwrap());

        let display = Rc::new(display);
        let target_texture = Texture2d::empty_with_format(
            &*display,
            glium::texture::UncompressedFloatFormat::U8U8U8U8,
            glium::texture::MipmapsOption::NoMipmap,
            dims.0,
            dims.1,
        )?;
        let target_depth = Texture2d::empty_with_format(
            &*display,
            glium::texture::UncompressedFloatFormat::F32,
            glium::texture::MipmapsOption::NoMipmap,
            dims.0,
            dims.1,
        )?;

        let raw_image = RawImage2d::from_raw_rgba_reversed(&image.to_vec(), dims);
        target_texture.write(
            glium::Rect {
                left: 0,
                bottom: 0,
                width: dims.0,
                height: dims.1,
            },
            raw_image,
        );
        let raw_depth = RawImage2d {
            data: Cow::Owned(image::imageops::flip_vertical(&depth).to_vec()),
            format: glium::texture::ClientFormat::U8,
            width: dims.0,
            height: dims.1,
        };
        target_depth.write(
            glium::Rect {
                left: 0,
                bottom: 0,
                width: dims.0,
                height: dims.1,
            },
            raw_depth,
        );

        let background_shader = if background_filling {
            Some(BackgroundShader::new(display.clone(), dims)?)
        } else {
            None
        };

        Ok(Self {
            display,
            program,
            vertex_buffer,
            target_texture,
            target_depth,
            view_params,
            background_shader,
            raster,
        })
    }

    fn render_to<S: glium::Surface>(&self, target: &mut S) {
        target.clear_depth(1.0);
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        let uniforms = uniform! {
            projectionview: *(self.view_params.projection * self.view_params.camera).as_ref(),
        };
        let mut draw_options = DrawParameters::default();
        draw_options.depth.test = glium::draw_parameters::DepthTest::IfLessOrEqual;
        draw_options.depth.write = true;
        draw_options.point_size = Some(1.0);
        target
            .draw(
                &self.vertex_buffer,
                &glium::index::NoIndices(glium::index::PrimitiveType::Points),
                &self.program,
                &uniforms,
                &draw_options,
            )
            .unwrap();
    }

    // TODO: remove toggle
    pub fn render(&mut self, toggle: bool) -> Result<(), Box<dyn std::error::Error>> {
        let target = self.display.draw();
        if self.raster {
            let dims = target.get_dimensions();
            // TODO: don't create new textures on every render iteration
            let depth_buffer = DepthTexture2d::empty(&*self.display, dims.0, dims.1)?;

            let outputs = [
                ("color_out", &self.target_texture),
                ("depth_out", &self.target_depth),
            ];
            let mut framebuffer = MultiOutputFrameBuffer::with_depth_buffer(
                &*self.display,
                outputs.iter().cloned(),
                &depth_buffer,
            )?;
            self.render_to(&mut framebuffer);
        }

        if let Some(background_shader) = &mut self.background_shader {
            if toggle {
                background_shader.run(&self.target_texture, &self.target_depth)?;
                let (color, _depth) = background_shader.front_buffer();
                color.sync_shader_writes_for_surface();
                color
                    .as_surface()
                    .fill(&target, glium::uniforms::MagnifySamplerFilter::Nearest);
            }
        }
        if !toggle {
            // Multi-output framebuffers don't support fill()
            let simple_buffer = SimpleFrameBuffer::new(&*self.display, &self.target_texture)?;
            simple_buffer.fill(&target, glium::uniforms::MagnifySamplerFilter::Nearest);
        }
        target.finish()?;

        Ok(())
    }

    pub fn save_screenshot(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let image: RawImage2d<'_, u8> = self.display.read_front_buffer()?;
        let image_buffer =
            ImageBuffer::from_raw(image.width, image.height, image.data.into_owned()).unwrap();
        let image = image::DynamicImage::ImageRgba8(image_buffer).flipv();
        image.save(name)?;

        Ok(())
    }
    pub fn save_depth(&self, name: &str) {
        let depth_texture = if let Some(background_shader) = self.background_shader.as_ref() {
            &background_shader.front_buffer().1
        } else {
            eprintln!("WARNING: Background shading disabled. Reading depth map from target depth");
            &self.target_depth
        };
        unsafe {
            let output: RawImage2d<'static, f32> =
                depth_texture.unchecked_read::<RawImage2d<'static, f32>, f32>();
            let image_buffer: ImageBuffer<image::Luma<u8>, Vec<u8>> = ImageBuffer::from_vec(output.width, output.height,
            output.data.iter().copied().map(|f| (f * 255.0) as u8).collect::<Vec<u8>>()).unwrap();

            let image = image::DynamicImage::ImageLuma8(image_buffer).flipv();

            image.save(name).unwrap();
        }
    }
}
