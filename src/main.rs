use headless::HeadlessRenderer;
use image::{io::Reader as ImageReader, ImageBuffer, Luma, Rgba};

use clap::Parser;
use glium::{glutin::surface::WindowSurface, Display};
use nalgebra::Vector3;
use renderer::Renderer;
use winit::{
    event::{Event, WindowEvent},
    window::Window,
};

mod background_shader;
mod filling_shader_wgpu;
mod headless;
mod renderer;
mod texture;
mod view_params;
mod wgpu_renderer;

fn open_display(
    event_loop: &winit::event_loop::EventLoop<()>,
    width: u32,
    height: u32,
) -> (Window, Display<WindowSurface>) {
    // Boilerplate code ripped from glium git
    use glutin::display::GetGlDisplay;
    use glutin::prelude::*;
    use raw_window_handle::HasRawWindowHandle;

    // First we start by opening a new Window
    let builder = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(width, height));
    let display_builder = glutin_winit::DisplayBuilder::new().with_window_builder(Some(builder));
    let config_template_builder = glutin::config::ConfigTemplateBuilder::new();

    let (window, gl_config) = display_builder
        .build(&event_loop, config_template_builder, |mut configs| {
            // Just use the first configuration since we don't have any special preferences here
            configs.next().unwrap()
        })
        .unwrap();
    let window = window.unwrap();

    // Now we get the window size to use as the initial size of the Surface
    let (width, height): (u32, u32) = window.inner_size().into();
    let attrs = glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
        .build(
            window.raw_window_handle(),
            std::num::NonZeroU32::new(width).unwrap(),
            std::num::NonZeroU32::new(height).unwrap(),
        );

    // Finally we can create a Surface, use it to make a PossiblyCurrentContext and create the glium Display
    let surface = unsafe {
        gl_config
            .display()
            .create_window_surface(&gl_config, &attrs)
            .unwrap()
    };
    let context_attributes = glutin::context::ContextAttributesBuilder::new()
        .with_context_api(glutin::context::ContextApi::OpenGl(Some(
            glutin::context::Version { major: 4, minor: 6 },
        )))
        .build(Some(window.raw_window_handle()));
    let current_context = Some(unsafe {
        gl_config
            .display()
            .create_context(&gl_config, &context_attributes)
            .expect("failed to create context")
    })
    .unwrap()
    .make_current(&surface)
    .unwrap();
    let display = Display::from_context_surface(current_context, surface).unwrap();
    (window, display)
}

#[derive(Parser)]
struct Args {
    #[arg(long)]
    headless: bool,
    image_path: String,
    depth_path: String,
    before_path: Option<String>,
    mask_path: Option<String>,
}

fn get_image(
    args: &Args,
) -> Result<
    (
        ImageBuffer<Rgba<u8>, Vec<u8>>,
        ImageBuffer<Luma<u8>, Vec<u8>>,
    ),
    Box<dyn std::error::Error>,
> {
    let img = ImageReader::open(&args.image_path)?.decode()?.to_rgba8();
    let mut depth = ImageReader::open(&args.depth_path)?.decode()?.to_luma8();
    //depth.save("/tmp/foo.png")?;
    assert_eq!(img.dimensions(), depth.dimensions());

    let mut test_image: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
        ImageBuffer::new(img.dimensions().0, img.dimensions().1);

    if let Some(before_path) = &args.before_path {
        if let Some(mask_path) = &args.mask_path {
            let before = ImageReader::open(&before_path)?.decode()?.to_rgba8();
            let mask = ImageReader::open(&mask_path)?.decode()?.to_luma8();
            for (i, (((maskrow, beforerow), afterrow), depthrow)) in mask
                .rows()
                .zip(before.rows())
                .zip(img.rows())
                .zip(depth.rows_mut())
                .enumerate()
            {
                for (j, (((mask, before), after), depth)) in maskrow
                    .zip(beforerow)
                    .zip(afterrow)
                    .zip(depthrow)
                    .enumerate()
                {
                    let beforev =
                        Vector3::new(before.0[0] as f32, before.0[1] as f32, before.0[2] as f32);
                    let afterv =
                        Vector3::new(after.0[0] as f32, after.0[1] as f32, after.0[2] as f32);
                    if (afterv - beforev).abs().magnitude() < 30.0 && mask.0[0] > 200 {
                        if mask.0[0] > 200 {
                            depth.0[0] = 0;
                            test_image.get_pixel_mut(j as u32, i as u32).0[0] = 255;
                        } else {
                            // Max depth to avoid background shading, probably a better way to do this by adding a mask input to the compute shader
                            depth.0[0] = 255;
                            test_image.get_pixel_mut(j as u32, i as u32).0[1] = 255;
                        }
                    }
                }
            }
        }
    }
    //test_image.save("/tmp/foo2.png")?;
    Ok((img, depth))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let (image, depth) = get_image(&args).unwrap();
    let dims = image.dimensions();

    let events_loop = winit::event_loop::EventLoopBuilder::new().build();

    let window = if !args.headless {
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(dims.0, dims.1))
            .build(&events_loop)?;
        //let (window, display) = open_display(&events_loop, dims.0, dims.1);
        Some(window)
    } else {
        None
    };

    let mut renderer = pollster::block_on(wgpu_renderer::Renderer::new(
        window,
        image.clone(),
        depth.clone(),
        true,
        true,
    ));

    if args.headless {
        let mut headless_renderer = HeadlessRenderer::new(renderer);
        headless_renderer.run()?;
    } else {
        let mut changed = true;
        let mut img_count = 0;
        let mut background_shading_enabled = true;
        let mut occlusion_shading_enabled = true;

        events_loop.run(move |e, _, ctrl| match e {
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('a'),
                ..
            } => {
                renderer
                    .view_params
                    .set_pitch(renderer.view_params.pitch() + 0.01);
                renderer.update_camera();
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('d'),
                ..
            } => {
                renderer
                    .view_params
                    .set_pitch(renderer.view_params.pitch() - 0.01);
                renderer.update_camera();
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('q'),
                ..
            } => {
                renderer
                    .view_params
                    .set_yaw(renderer.view_params.yaw() + 0.01);
                renderer.update_camera();
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('e'),
                ..
            } => {
                renderer
                    .view_params
                    .set_yaw(renderer.view_params.yaw() - 0.01);
                renderer.update_camera();
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('w'),
                ..
            } => {
                renderer
                    .view_params
                    .set_roll(renderer.view_params.roll() + 0.01);
                renderer.update_camera();
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('s'),
                ..
            } => {
                renderer
                    .view_params
                    .set_roll(renderer.view_params.roll() - 0.01);
                renderer.update_camera();
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('f'),
                ..
            } => {
                let now = std::time::Instant::now();
                renderer
                    .save_screenshot(&format!("screenshot-{img_count}.png"))
                    .unwrap();
                println!(
                    "Screenshot saved to screenshot-{img_count}.png in {:?}",
                    std::time::Instant::now() - now
                );
                img_count += 1;
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('t'),
                ..
            } => {
                background_shading_enabled = !background_shading_enabled;
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('y'),
                ..
            } => {
                // enable background filling
                occlusion_shading_enabled = !occlusion_shading_enabled;
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('['),
                ..
            } => {
                // enable background filling
                renderer.background_shading_iters =
                    std::cmp::max(1, renderer.background_shading_iters.saturating_sub(1));
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(']'),
                ..
            } => {
                // enable background filling
                renderer.background_shading_iters =
                    renderer.background_shading_iters.saturating_add(1);
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(';'),
                ..
            } => {
                // enable background filling
                renderer.occlusion_shading_iters =
                    std::cmp::max(1, renderer.occlusion_shading_iters.saturating_sub(1));
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter('\''),
                ..
            } => {
                // enable background filling
                renderer.occlusion_shading_iters =
                    renderer.occlusion_shading_iters.saturating_add(1);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => ctrl.set_exit_with_code(0),

            Event::RedrawRequested(..) => {
                renderer.render(background_shading_enabled, occlusion_shading_enabled).unwrap();
            }

            Event::MainEventsCleared => {
                renderer
                    .head_state
                    .as_ref()
                    .unwrap()
                    .window
                    .request_redraw();
            }
            _ => {}
        });
    }
    Ok(())
}
