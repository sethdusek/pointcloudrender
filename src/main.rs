use image::{io::Reader as ImageReader, GrayImage, ImageBuffer, Luma, Rgb, Rgba};

use glium::{
    glutin::event::{Event, WindowEvent},
    implement_vertex,
    texture::{CompressedSrgbTexture2d, SrgbTexture2d, UnsignedTexture2d},
    uniform,
    uniforms::{ImageUnit, ImageUnitFormat, UniformBuffer},
    DrawParameters, IndexBuffer, Program, Rect, Surface, Texture2d, VertexBuffer,
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
        .with_srgb(false);

    let display = glium::Display::new(wb, cb, &events_loop).unwrap();

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
                    ((c2.0[0] - min_depth) as f32 / max_depth) * -2.0 + 1.0,
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
    dbg!(vertices
        .iter()
        .map(|v| float_ord::FloatOrd(v.position[2]))
        .max()
        .unwrap());
    dbg!(vertices
        .iter()
        .map(|v| float_ord::FloatOrd(v.position[2]))
        .min()
        .unwrap());

    let mut eye = Point3::new(0.0f32, 0.0, 1.0);
    let mut look_at = Point3::new(0.0, 0.0, -0.1);
    let vertex_buffer = VertexBuffer::new(&display, &vertices)?;
    let mut view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
    // TODO: figure out projection. This is just a placeholder
    let mut projection = Matrix4::<f32>::identity();
    let fov = 90.0;
    //let mut projection = Perspective3::new(
    //     dims.0 as f32 / dims.1 as f32,
    //     fov / (2.0 * std::f32::consts::PI),
    //     0.0,
    //     999.0,
    // );

    // don't think we even need this. but as you can tell by all the other commented out projection stuff im losing my sanity
    let mut projection = Matrix4::new_orthographic(-1.0f32, 1.0, -1.0, 1.0, 0.0, 999.0);

    events_loop.run(move |e, _, ctrl| match e {
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('a'),
            ..
        } => {
            eye.x -= 0.01;
            view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('d'),
            ..
        } => {
            eye.x += 0.01;
            view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('w'),
            ..
        } => {
            eye.z += 0.01;
            view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('s'),
            ..
        } => {
            eye.z -= 0.01;
            view = Matrix4::look_at_rh(&eye, &look_at, &Vector3::new(0.0, 1.0, 0.0));
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('q'),
            ..
        } => {
            //projection.set_fovy(projection.fovy() - 0.01);
        }
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('e'),
            ..
        } => {
            //projection.set_fovy(projection.fovy() + 0.01);
        }

        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => ctrl.set_exit_with_code(0),

        Event::MainEventsCleared => {
            // dbg!(projection);
            // println!("Enter args: ");
            // let mut buf = String::new();
            // std::io::stdin().read_line(&mut buf).unwrap();
            // let (znear, zfar) = buf.trim().split_once(" ").unwrap();
            //projection = Matrix4::new_perspective(dims.0 as f32 / dims.1 as f32, 45.0, znear.parse::<f32>().unwrap(), zfar.parse::<f32>().unwrap());
            let mut target = display.draw();
            target.clear_depth(1.0);
            target.clear_color(0.0, 0.0, 0.0, 1.0);

            let uniforms = uniform! {
                view: *view.as_ref(),
                projection: *projection.as_ref()
            };
            let mut draw_options = DrawParameters::default();
            draw_options.depth.test = glium::draw_parameters::DepthTest::IfLessOrEqual;
            draw_options.depth.write = true;
            target
                .draw(
                    &vertex_buffer,
                    &glium::index::NoIndices(glium::index::PrimitiveType::Points),
                    &program,
                    &uniforms,
                    &draw_options,
                )
                .unwrap();
            target.finish().unwrap();
        }
        _ => {}
    });
}
