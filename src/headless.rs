use crate::wgpu_renderer::Renderer;
use std::io::prelude::*;

pub struct HeadlessRenderer {
    renderer: Renderer,
    stdin: std::io::StdinLock<'static>,
    buf: String,
}

fn parse_num(num: &str) -> Result<f32, Box<dyn std::error::Error>> {
    match num.parse::<f32>() {
        Ok(num) => Ok(num),
        Err(_) if num.len() >= 2 => Ok(num[1..].parse::<f32>()?),
        Err(e) => Err(Box::new(e)),
    }
}

impl HeadlessRenderer {
    pub fn new(renderer: Renderer) -> Self {
        HeadlessRenderer {
            renderer,
            stdin: std::io::stdin().lock(),
            buf: String::new(),
        }
    }

    // TODO: use anyhow to simplify error handling across the entire codebase
    pub fn run(&mut self) -> Result<(), std::io::Error> {
        loop {
            self.buf.clear();
            self.stdin.read_line(&mut self.buf)?;
            self.parse_line().unwrap();
        }
    }

    pub fn parse_line(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self.buf.trim_end().split_once(" ") {
            Some(("yaw", num)) => {
                let num = parse_num(num)?;
                self.renderer
                    .view_params
                    .set_yaw(self.renderer.view_params.yaw() + num);
            }
            Some(("pitch", num)) => {
                let num = parse_num(num)?;
                self.renderer
                    .view_params
                    .set_pitch(self.renderer.view_params.pitch() + num);
            }
            Some(("roll", num)) => {
                let num = parse_num(num)?;
                self.renderer
                    .view_params
                    .set_roll(self.renderer.view_params.roll() + num);
            }

            Some(("screenshot", filename)) => {
                self.renderer.update_camera();
                self.renderer.render(true, true)?;
                self.renderer.save_screenshot(filename)?;
            }
            _ => (),
        }
        Ok(())
    }
}
