#![deny(unused_must_use)]
use anyhow::Context as ContextTrait;
use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};
#[derive(Default)]
struct App<'a> {
    window: Option<Window>,
    context: Option<Context<&'a Window>>,
    surface: Option<Surface<&'a Window, &'a Window>>,
}

impl<'a> ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
                        .with_decorations(false)
                        .with_title("HEYYYYYYYYY"),
                )
                .unwrap(),
        );
        // one of these two unsafe blocks causes a double free and a SIGSEGV
        // solution: make a new wrapper struct ?
        self.context = unsafe {
            // we know the window will be valid for as long as the struct, since we just created it!
            // lifetime trickery :(
            std::mem::transmute(Some(Context::new(self.window.as_ref().unwrap()).unwrap()))
        };

        self.surface = unsafe {
            std::mem::transmute(Some(
                Surface::new(
                    self.context.as_ref().unwrap(),
                    self.window.as_ref().unwrap(),
                )
                .unwrap(),
            ))
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested if matches!(self.window.as_ref(), Some(..)) => 'redraw: {
                let window = self.window.as_mut().unwrap();
                let surface = self.surface.as_mut().unwrap();
                let context = self.context.as_ref().unwrap();

                if window.id() != id {
                    break 'redraw;
                }
                window.set_decorations(false);
                window
                    .set_cursor_hittest(false)
                    .expect("Should allow passthrough");
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.
                let (width, height) = {
                    let size = window.inner_size();
                    (size.width, size.height)
                };

                surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();

                let mut pixmap = Pixmap::new(width, height).unwrap();
                pixmap.fill(Color::WHITE);
                let path = PathBuilder::from_circle(
                    (width / 2) as f32,
                    (height / 2) as f32,
                    (width.min(height) / 2) as f32,
                )
                .unwrap();
                let mut paint = Paint::default();
                paint.set_color_rgba8(255, 128, 128, 128);
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::EvenOdd,
                    Transform::identity(),
                    None,
                );
                paint.set_color_rgba8(255, 0, 0, 255);
                let mut stroke = Stroke::default();
                stroke.width = 10.0;
                pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);

                let mut buffer = surface.buffer_mut().unwrap();
                for index in 0..(width * height) as usize {
                    buffer[index] = pixmap.data()[index * 4 + 2] as u32
                        | (pixmap.data()[index * 4 + 1] as u32) << 8
                        | (pixmap.data()[index * 4] as u32) << 16;
                }

                buffer.present().unwrap();
                // Draw.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().unwrap();

    // This Leaks Memory
    let mut app = App::default();
    event_loop
        .run_app(&mut app)
        .context("Event loop broke running app")?;

    // If we do not forcibly exit, drop(app) gets called and SIGSEGV happens
    // App is memory unsafe - we must rely on the OS to free it when we are done
    // Could refactor to fix app's memory unsafety but lowkey not feeling it right now
    std::process::exit(0);
}
