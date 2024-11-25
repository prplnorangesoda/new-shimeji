#![deny(unused_must_use)]
#![allow(dead_code)]
use anyhow::Context as _;
use deadpool::unmanaged::Pool;
use std::{
    num::NonZeroU32,
    process::exit,
    rc::Rc,
    sync::Arc,
    sync::{atomic::AtomicBool, mpsc},
    thread,
    time::Duration,
};
use tao::{
    dpi::{LogicalSize, Size},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod rgba;
mod shimeji;

use rgba::RGBA;
use shimeji::ShimejiBucket;

#[derive(Debug)]
enum Status {
    Ok,
    Exiting,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Going");

    let should_exit = Arc::new(AtomicBool::new(false));

    let copy = should_exit.clone();

    let parallelism = std::thread::available_parallelism()?.get();

    let mut shimeji_vec = vec![];
    for i in 1..=parallelism {
        shimeji_vec.push(ShimejiBucket::new())
    }

    println!("Available parallelism: {}", parallelism);
    let running_thread = thread::spawn(move || {
        while !copy.load(std::sync::atomic::Ordering::Relaxed) {
            println!("HI I AM THE MULTI THREAD");

            thread::sleep(Duration::from_millis(500));
        }
    });

    thread::sleep(Duration::from_secs(5));

    should_exit.store(true, std::sync::atomic::Ordering::SeqCst);
    running_thread.join().ok();
    Ok(())
    // let event_loop = EventLoop::new();
    // let window = WindowBuilder::new()
    //     .with_decorations(true)
    //     .with_transparent(true)
    //     .with_always_on_top(true)
    //     .with_min_inner_size(LogicalSize::new(100, 100))
    //     .build(&event_loop)
    //     .context("Building initial window failed")?;

    // let (window, _context, mut surface) = {
    //     let window = Rc::new(window);
    //     let context = softbuffer::Context::new(window.clone()).unwrap();
    //     let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    //     (window, context, surface)
    // };

    // println!("Current Monitor: {:?}", window.current_monitor());
    // window.set_title("Awesome window!");
    // window
    //     .set_ignore_cursor_events(true)
    //     .expect("Should be possible to ignore cursor events");
    // // if cfg!(target_os = "linux") {
    // //     native_dialog::MessageDialog::new()
    // //         .set_title("info")
    // //         .set_type(native_dialog::MessageType::Warning)
    // //         .set_text("If you're on Wayland, you must set this window to show on top specifically.")
    // //         .show_alert()
    // //         .ok();
    // // }

    // event_loop.run(move |event, _, control_flow| {
    //     *control_flow = ControlFlow::Wait;
    //     // println!("{event:?}");

    //     match event {
    //         Event::LoopDestroyed => {
    //             println!("Bye!")
    //         }
    //         Event::WindowEvent {
    //             event: WindowEvent::CloseRequested,
    //             ..
    //         } => {
    //             *control_flow = ControlFlow::Exit;
    //         }

    //         Event::RedrawRequested(_) => {
    //             let (width, height) = {
    //                 let size = window.inner_size();
    //                 (size.width, size.height)
    //             };
    //             surface
    //                 .resize(
    //                     NonZeroU32::new(width).unwrap(),
    //                     NonZeroU32::new(height).unwrap(),
    //                 )
    //                 .unwrap();

    //             let mut buffer = surface.buffer_mut().unwrap();
    //             println!("Buffer length: {}", buffer.len());
    //             println!(
    //                 "First four buffer bytes: {:b} {:b} {:b} {:b}",
    //                 buffer[0], buffer[1], buffer[2], buffer[3]
    //             );
    //             let color_u32 = RGBA::new(0, 0, 0, 10).to_softbuf_u32();
    //             buffer.fill(color_u32);
    //             buffer.present().unwrap();
    //         }

    //         _ => (),
    //     }
    // });
}
