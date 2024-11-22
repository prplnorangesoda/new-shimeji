use softbuffer::{Context, Surface};
use std::num::NonZeroU32;
use std::rc::Rc;
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId, WindowLevel};

#[derive(Default)]
pub struct App {
    window: Option<Rc<Window>>,
    context: Option<Context<Rc<Window>>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_visible(false)
                        .with_window_level(WindowLevel::AlwaysOnTop)
                        .with_decorations(false)
                        .with_transparent(true)
                        .with_title("HEYYYYYYYYY"),
                )
                .unwrap(),
        ));

        self.context = Some(Context::new(self.window.as_ref().unwrap().clone()).unwrap());

        self.surface = Some(
            Surface::new(
                self.context.as_ref().unwrap(),
                self.window.as_ref().unwrap().clone(),
            )
            .unwrap(),
        );
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested if matches!(self.window.as_ref(), Some(_)) => 'redraw: {
                let window = self.window.as_mut().unwrap();
                let surface = self.surface.as_mut().unwrap();

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
                pixmap.fill(Color::from_rgba8(0, 0, 0, 255));
                let path = PathBuilder::from_circle(
                    (width / 2) as f32,
                    (height / 2) as f32,
                    (width.min(height) / 2) as f32,
                )
                .unwrap();
                let mut paint = Paint::default();
                paint.set_color_rgba8(255, 128, 128, 255);
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
                window.set_visible(true);

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                // window.request_redraw();
            }
            _ => (),
        }
    }
}
