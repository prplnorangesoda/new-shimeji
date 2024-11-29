#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]
use anyhow::Context as _;
use deadpool::unmanaged::Pool;
use std::{
    num::NonZeroU32,
    process::exit,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
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
use shimeji::{BucketError, ShimejiBucket};

use derive_more::{derive::From, Display, Error};

#[derive(Debug)]
enum Status {
    Ok,
    Exiting,
}
#[derive(Display, Debug, Error, From)]
enum ManagerError {
    /// Should never happen.
    NoBucketsAvailable,
    BucketError(BucketError),
}

#[derive(Debug)]
struct BucketManager<'a> {
    buckets: Vec<ShimejiBucket<'a>>,
    should_exit: Arc<AtomicBool>,
}
impl BucketManager<'_> {
    pub fn new(amount: usize) -> Self {
        assert!(amount != 0);
        let mut buckets = Vec::with_capacity(amount);
        let should_exit = Arc::new(AtomicBool::new(false));
        for i in 0..amount {
            buckets.push(ShimejiBucket::new(i, should_exit.clone()))
        }
        Self {
            should_exit,
            buckets,
        }
    }
    pub fn add_shimeji<'a>(&mut self, _conf: &'a ShimejiConfig) -> Result<(), ManagerError> {
        let bucket = self
            .buckets
            .iter_mut()
            .reduce(|acc, bucket| {
                if bucket.len() > acc.len() {
                    bucket
                } else {
                    acc
                }
            })
            .ok_or(ManagerError::NoBucketsAvailable)?;

        bucket.add();
        Ok(())
    }
    pub fn run(mut self) -> Result<(), ManagerError> {
        for bucket in self.buckets.iter_mut() {
            bucket.init()?
        }
        loop {
            thread::sleep(Duration::from_secs(5));
            self.should_exit.store(true, Ordering::Release);
            for bucket in self.buckets.into_iter() {
                bucket.join_thread()?
            }
            break Ok(());
        }
    }
}

#[derive(Debug)]
pub struct ShimejiConfig {
    name: String,
}
fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .expect("Should be able to set up logger");
    log::info!("Going");

    let parallelism = std::thread::available_parallelism()
        .context("Failed to get available parallelism for this system")?
        .get();
    log::info!("Available parallelism: {}", parallelism);

    let mut manager = BucketManager::new(parallelism);

    let example_config = ShimejiConfig {
        name: String::from("Name"),
    };
    manager.add_shimeji(&example_config)?;

    manager.run()?;
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

mod test {
    #[test]
    fn shimejis_are_added_correctly() -> anyhow::Result<()> {
        Ok(())
    }
}
