use image::{io::Reader as ImageReader, GrayImage, ImageBuffer, Luma, Rgb, Rgba};

use glium::{
    glutin::event::{Event, WindowEvent},
    implement_vertex,
    texture::{CompressedSrgbTexture2d, SrgbTexture2d, UnsignedTexture2d, RawImage2d},
    uniform,
    uniforms::{ImageUnit, ImageUnitFormat, UniformBuffer},
    Display, DrawParameters, IndexBuffer, Program, Rect, Surface, Texture2d, VertexBuffer,
};
use nalgebra::{Matrix4, Perspective3, Point3, Vector3};
fn get_image() -> Result<
    (
        ImageBuffer<Rgba<u8>, Vec<u8>>,
        ImageBuffer<Luma<u8>, Vec<u8>>,
    ),
    Box<dyn std::error::Error>,
> {
    let mut args = std::env::args().skip(1);
    let img_path = args.next().unwrap();
    let depth_path = args.next().unwrap();
    let img = ImageReader::open(img_path)?.decode()?.to_rgba8();
    let depth = ImageReader::open(depth_path)?.decode()?.to_luma8();
    assert_eq!(img.dimensions(), depth.dimensions());
    Ok((img, depth))
}

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}
implement_vertex!(Vertex, position, color);

struct ViewParams {
    eye: Point3<f32>,
    look_at: Point3<f32>,
    camera: Matrix4<f32>,
    projection: Matrix4<f32>,
}

impl ViewParams {
    pub fn new(eye: Point3<f32>, look_at: Point3<f32>, projection: Matrix4<f32>) -> Self {
        ViewParams {
            eye,
            look_at,
            camera: Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0)),
            projection,
        }
    }

    fn update_camera(&mut self) {
        self.camera = Matrix4::look_at_rh(&self.eye, &self.look_at, &Vector3::new(0.0, 1.0, 0.0));
    }

    pub fn set_eye(&mut self, eye: Point3<f32>) {
        self.eye = eye;
        self.update_camera();
    }
    pub fn set_look_at(&mut self, look_at: Point3<f32>) {
        self.look_at = look_at;
        self.update_camera();
    }
}
struct Renderer {
    display: Display,
    program: Program,
    vertex_buffer: VertexBuffer<Vertex>,
    view_params: ViewParams,
}

impl Renderer {
    pub fn new(
        display: Display,
        image: ImageBuffer<Rgba<u8>, Vec<u8>>,
        depth: ImageBuffer<Luma<u8>, Vec<u8>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        assert_eq!(image.dimensions(), depth.dimensions());
        let dims = image.dimensions();
        let program = Program::from_source(
            &display,
            include_str!("vertex.glsl"),
            include_str!("fragment.glsl"),
            None,
        )?;
        let mut vertices = Vec::with_capacity((dims.0 * dims.1) as usize);
        let min_depth = depth.rows().flatten().map(|luma| luma.0[0]).min().unwrap();
        let max_depth = depth.rows().flatten().map(|luma| luma.0[0]).max().unwrap() as f32;
        // Generate vertices for each pixel. OpenGL coordinates have a minimum of -1 and maximum of 1
        for (y, (r1, r2)) in image.rows().zip(depth.rows()).enumerate() {
            for (x, (c1, c2)) in r1.zip(r2).enumerate() {
                vertices.push(Vertex {
                    position: [
                        (x as f32 / dims.0 as f32) * 2.0 - 1.0,
                        // Top of the screen is +1 in OpenGL
                        (y as f32 / dims.1 as f32) * -2.0 + 1.0,
                        ((c2.0[0] - min_depth) as f32 / max_depth) * -2.0 + 0.9,
                    ],
                    color: [
                        c1.0[0] as f32 / 255.0,
                        c1.0[1] as f32 / 255.0,
                        c1.0[2] as f32 / 255.0,
                        1.0,
                    ],
                });
            }
        }
        let vertex_buffer = VertexBuffer::new(&display, &vertices)?;

        let eye = Point3::new(0.0f32, 0.0, 1.0);
        let look_at = Point3::new(0.0, 0.0, -0.1);

        // TODO: figure out projection. This is just a placeholder
        let view_params = ViewParams::new(eye, look_at, Matrix4::new_orthographic(-1.0f32, 1.0, -1.0, 1.0, 0.0, 999.0));

        Ok(Self {
            display,
            program,
            vertex_buffer,
            view_params,
        })
    }

    fn render(&self) {
        let mut target = self.display.draw();
        target.clear_depth(1.0);
        target.clear_color(0.0, 0.0, 0.0, 1.0);

        let uniforms = uniform! {
            projectionview: *(self.view_params.projection * self.view_params.camera).as_ref(),
        };
        let mut draw_options = DrawParameters::default();
        draw_options.depth.test = glium::draw_parameters::DepthTest::IfLessOrEqual;
        draw_options.depth.write = true;
        draw_options.point_size = Some(2.0);
        target
            .draw(
                &self.vertex_buffer,
                &glium::index::NoIndices(glium::index::PrimitiveType::Points),
                &self.program,
                &uniforms,
                &draw_options,
            )
            .unwrap();
        target.finish().unwrap();
    }
    fn save_screenshot(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let image: RawImage2d<'_, u8> = self.display.read_front_buffer()?;
        dbg!(image.width, image.height);
        let image_buffer = ImageBuffer::from_raw(image.width, image.height, image.data.into_owned()).unwrap();
        let image = image::DynamicImage::ImageRgba8(image_buffer).flipv();
        image.save(name)?;

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (image, depth) = get_image().unwrap();
    let dims = image.dimensions();
    //let dims = (640, 640);

    let events_loop = glium::glutin::event_loop::EventLoop::new();
    let wb = glium::glutin::window::WindowBuilder::new()
        .with_inner_size(glium::glutin::dpi::PhysicalSize::new(dims.0, dims.1))
        .with_resizable(false)
        .with_title("Point Cloud Render");

    let cb = glium::glutin::ContextBuilder::new()
        .with_gl(glium::glutin::GlRequest::Latest)
        .with_pixel_format(8, 8)
        .with_srgb(true);

    let display = glium::Display::new(wb, cb, &events_loop).unwrap();

    let mut renderer = Renderer::new(display, image, depth)?;

    let mut changed = true;
    let mut img_count = 0;
    events_loop.run(move |e, _, ctrl| match e {
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('a'),
            ..
        } => {
            renderer.view_params.set_eye(renderer.view_params.eye + Vector3::new(-0.01, 0.0, 0.0));
            changed = true;
            //renderer.view_params.set_look_at(renderer.view_params.look_at + Vector3::new(-0.01, 0.0, 0.0));
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('d'),
            ..
        } => {
            renderer.view_params.set_eye(renderer.view_params.eye + Vector3::new(0.01, 0.0, 0.0));
            changed = true;
            // eye.x += 0.01;
            // view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('w'),
            ..
        } => {
            // eye.z += 0.01;
            // view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
            changed = true;
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('s'),
            ..
        } => {
            changed = true;
            // eye.z -= 0.01;
            // view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => ctrl.set_exit_with_code(0),

        Event::MainEventsCleared => {
            renderer.render();
            if changed {
                renderer.save_screenshot(&format!("screenshot-{}.png", img_count)).unwrap();
                img_count+=1;
                changed = false;
            }
        }
        _ => {}
    });
}
