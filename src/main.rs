#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]
use anyhow::Context as _;
use std::{
    fs::File,
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

use rgba::Rgba;
use shimeji::{BucketError, Shimeji, ShimejiBucket};

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
struct BucketManager {
    buckets: Vec<ShimejiBucket>,
    should_exit: Arc<AtomicBool>,
}
impl BucketManager {
    ///
    /// # Panics
    /// Panics if `amount == 0`.
    pub fn new(amount: usize) -> Self {
        assert!(amount != 0);
        let mut buckets = Vec::with_capacity(amount);
        let should_exit = Arc::new(AtomicBool::new(false));
        for i in 0..amount {
            let mut bucket = ShimejiBucket::new(i, should_exit.clone());
            bucket.init().expect("should be able to init bucket");
            buckets.push(bucket);
        }
        Self {
            should_exit,
            buckets,
        }
    }
    pub fn add_shimeji(&mut self, conf: &ShimejiConfig) -> Result<(), ManagerError> {
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

        log::info!("Adding a new shimeji to bucket id: {}", bucket.id);

        let shimeji = Shimeji::with_config(conf);
        bucket.add(shimeji)?;
        Ok(())
    }
    pub fn run(mut self, mut tray_handle: tray_item::TrayItem) -> Result<(), ManagerError> {
        let example_config = ShimejiConfig {
            name: String::from("Name"),
        };
        self.add_shimeji(&example_config)?;
        tray_handle.add_label("label").unwrap();

        loop {
            log::trace!("Yielding main thread");
            thread::sleep(Duration::from_secs(0));
            thread::yield_now();
        }
    }
}

#[derive(Debug)]
pub struct ShimejiConfig {
    name: String,
}
fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .env()
        .init()
        .expect("Should be able to set up logger");
    log::info!("Going");

    let parallelism = thread::available_parallelism()
        .context("Failed to get available parallelism for this system")?
        .get();
    log::info!("Available parallelism: {}", parallelism);

    let manager = BucketManager::new(parallelism);

    let path = std::env!("HOME").to_owned() + "/tray_icon-red.png";

    dbg!(&path);
    let decoder_red = png::Decoder::new(File::open(&path).unwrap());
    let (info_red, mut reader_red) = decoder_red.read_info().unwrap();
    let mut buf_red = vec![0; info_red.buffer_size()];
    reader_red.next_frame(&mut buf_red).unwrap();

    let icon_red = tray_item::IconSource::Data {
        data: buf_red,
        height: 32,
        width: 32,
    };

    let tray_handle = tray_item::TrayItem::new("Example", icon_red).unwrap();
    manager.run(tray_handle)?;
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
