
use glium::glutin;
use glium::glutin::event::{Event, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontGlyphRanges, FontSource, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::path::Path;
use std::time::Instant;

use glutin::event::KeyboardInput;

pub mod game;
pub use game::*;




struct System {
    event_loop: EventLoop<()>,
    display: glium::Display,
    imgui: Context,
    platform: WinitPlatform,
    renderer: Renderer,
    font_size: f32,
}

pub struct MyRenderer
{
    system: System,
}




pub trait Updater {
    fn update(&mut self, ui: &Ui, screen_sz: &[f32; 2]);
}

impl MyRenderer
{
    
    pub fn new(wnd_name: &str) -> MyRenderer
    {
        let event_loop = glutin::event_loop::EventLoop::new();
        let wb = glutin::window::WindowBuilder::new().with_title(wnd_name)
            .with_inner_size(glutin::dpi::LogicalSize::new(1024f64, 768f64));
        let cb = glutin::ContextBuilder::new().with_vsync(true);
        let display = glium::Display::new(wb, cb, &event_loop).expect("Failed to initialize display");


        let mut imgui = Context::create();
        imgui.set_ini_filename(None);
        

        let mut platform = WinitPlatform::init(&mut imgui);
        {
            let gl_window = display.gl_window();
            let window = gl_window.window();

            let dpi_mode = if let Ok(factor) = std::env::var("IMGUI_EXAMPLE_FORCE_DPI_FACTOR") {
                // Allow forcing of HiDPI factor for debugging purposes
                match factor.parse::<f64>() {
                    Ok(f) => HiDpiMode::Locked(f),
                    Err(e) => panic!("Invalid scaling factor: {}", e),
                }
            } else {
                HiDpiMode::Default
            };

            platform.attach_window(imgui.io_mut(), window, dpi_mode);
        }

        let font_size = 13.0;

        imgui.fonts().add_font(&[
            FontSource::TtfData {
                data: include_bytes!("../../resources/Roboto-Regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    rasterizer_multiply: 1.5,
                    oversample_h: 4,
                    oversample_v: 4,
                    ..FontConfig::default()
                }),
            },
            FontSource::TtfData {
                data: include_bytes!("../../resources/mplus-1p-regular.ttf"),
                size_pixels: font_size,
                config: Some(FontConfig {
                    oversample_h: 4,
                    oversample_v: 4,
                    glyph_ranges: FontGlyphRanges::japanese(),
                    ..FontConfig::default()
                }),
            },
        ]);
        let renderer = Renderer::init(&mut imgui, &display).expect("Failed to initialize renderer");
        MyRenderer{ system: System{
                renderer: renderer,
                imgui: imgui,
                event_loop: event_loop,
                display: display,
                platform: platform,
                font_size: font_size,
            }
        }
    }
    pub fn run<F: FnMut(&mut bool, &mut Ui, &[f32; 2]) + 'static>(mut self, mut run_ui: F)
    {
        let System {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self.system;
        let mut last_frame = Instant::now();

        event_loop.run(move |event, _, control_flow| match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;
            }
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();
                platform
                    .prepare_frame(imgui.io_mut(), gl_window.window())
                    .expect("Failed to prepare frame");
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                let mut ui = imgui.frame();

                let mut run = true;

                let sz = display.get_framebuffer_dimensions();
                let screen_sz = [sz.0 as f32, sz.1 as f32];

                run_ui(&mut run, &mut ui, &screen_sz);
               
                if !run {
                    *control_flow = ControlFlow::Exit;
                }

                let gl_window = display.gl_window();
                let mut target = display.draw();
                target.clear_color_srgb(0.2, 0.2, 0.2, 1.0);
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("Rendering failed");
                target.finish().expect("Failed to swap buffers");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            event => {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        })
    }
    pub fn add_quad(&mut self, pos_start: &[f32; 2], pos_size: &[f32; 2], uv_start: &[f32; 2], uv_size: &[f32; 2], base_color: &[f32; 4])
    {
        
    }
    pub fn add_player(&mut self, player: &Player)
    {
        self.add_quad(&player.pos, &[100.0, 100.0], &[0.0, 0.0f32], &[0.0, 0.0f32], &player.col);
    }



}