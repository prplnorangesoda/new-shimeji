#![deny(unused_must_use)]
use anyhow::Context as _;
use std::{num::NonZeroU32, rc::Rc};
use tao::{
    dpi::{LogicalSize, Size},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct RGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl RGBA {
    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        RGBA {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn from_tuple<T>(tuple: T) -> Self
    where
        T: Into<(u8, u8, u8, u8)>,
    {
        let rgba = tuple.into();
        RGBA {
            red: rgba.0,
            green: rgba.1,
            blue: rgba.2,
            alpha: rgba.3,
        }
    }

    /// --------
    ///
    /// Pixel format (`u32`):
    ///
    /// 00000000RRRRRRRRGGGGGGGGBBBBBBBB
    ///
    /// 0: Bit is 0
    /// R: Red channel
    /// G: Green channel
    /// B: Blue channel
    pub fn to_softbuf_u32(&self) -> u32 {
        if self.alpha == 0 {
            return 0;
        }
        // println!("self.alpha as f32: {}", self.alpha as f32);
        // let alpha = self.alpha as f32 / 255.0;
        // let red = (self.red as f32 * alpha).floor() as u32;
        // let blue = (self.blue as f32 * alpha).floor() as u32;
        // let green = (self.green as f32 * alpha).floor() as u32;
        // println!("red: {}", red);
        // println!("green: {}", green);
        // println!("blue: {}", blue);
        // println!("alpha: {}", alpha);

        // let ret = (red << 16) | (blue << 8) | green;

        // println!("ret: {:#034b}", ret);
        // ret

        (self.alpha as u32) << 24
            | (self.red as u32) << 16
            | (self.green as u32) << 8
            | self.blue as u32
    }
}
fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_transparent(true)
        .with_always_on_top(true)
        .with_min_inner_size(LogicalSize::new(100, 100))
        .build(&event_loop)
        .context("Building initial window failed")?;

    let (window, _context, mut surface) = {
        let window = Rc::new(window);
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
        (window, context, surface)
    };

    println!("Current Monitor: {:?}", window.current_monitor());
    window.set_title("Awesome window!");
    window
        .set_ignore_cursor_events(true)
        .expect("Should be possible to ignore cursor events");
    // if cfg!(target_os = "linux") {
    //     native_dialog::MessageDialog::new()
    //         .set_title("info")
    //         .set_type(native_dialog::MessageType::Warning)
    //         .set_text("If you're on Wayland, you must set this window to show on top specifically.")
    //         .show_alert()
    //         .ok();
    // }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        // println!("{event:?}");

        match event {
            Event::LoopDestroyed => {
                println!("Bye!")
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::RedrawRequested(_) => {
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

                let mut buffer = surface.buffer_mut().unwrap();
                println!("Buffer length: {}", buffer.len());
                println!(
                    "First four buffer bytes: {:b} {:b} {:b} {:b}",
                    buffer[0], buffer[1], buffer[2], buffer[3]
                );
                let color_u32 = RGBA::new(0, 0, 0, 100).to_softbuf_u32();
                buffer.fill(color_u32);
                buffer.present().unwrap();
            }

            _ => (),
        }
    });
}
