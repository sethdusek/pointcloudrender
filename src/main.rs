use image::{io::Reader as ImageReader, GrayImage, ImageBuffer, Luma, Rgb, Rgba};

use glium::{
    glutin::event::{Event, WindowEvent},
    implement_vertex,
    texture::{CompressedSrgbTexture2d, SrgbTexture2d, UnsignedTexture2d},
    uniform,
    uniforms::{ImageUnit, ImageUnitFormat, UniformBuffer},
    IndexBuffer, Program, Rect, Surface, Texture2d, VertexBuffer,
};
use nalgebra::{Matrix4, Vector3};
fn get_image() -> Result<(ImageBuffer<Rgba<u8>, Vec<u8>>, ImageBuffer<Luma<u8>, Vec<u8>>), Box<dyn std::error::Error>> {
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
                    // Top of the screen is +1 in OpenGL. Thus we want the first row of the image to be at (..., 1, ...) and the last row to be (..., -1, ...)
                    (y as f32 / dims.1 as f32) * -2.0 + 1.0,
                    ((c2.0[0] - min_depth) as f32 / max_depth) * -2.0 + 1.0
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
    let mut translation = Vector3::new(0.0f32, 0.0, 0.0);
    let mut view = Matrix4::new_translation(&translation);

    events_loop.run(move |e, _, ctrl| match e {
        Event::WindowEvent {
            event: WindowEvent::ReceivedCharacter('w'),
            ..
        } => {
            translation.z -= 0.01;
            view = Matrix4::new_translation(&translation);
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => ctrl.set_exit_with_code(0),

        Event::MainEventsCleared => {
            dbg!("Drawing");
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);

            let uniforms = uniform! {
                view: *view.as_ref()
            };
            target
                .draw(
                    &vertex_buffer,
                    &glium::index::NoIndices(glium::index::PrimitiveType::Points),
                    &program,
                    &uniforms,
                    &Default::default(),
                )
                .unwrap();
            target.finish().unwrap();
        },
        _ => {}
    });
}
