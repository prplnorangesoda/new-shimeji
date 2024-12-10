#![deny(unused_must_use)]
#![allow(dead_code)]
#![allow(unused_imports)]

use anyhow::Context as _;
use itertools::Itertools;
use std::{
    cell::Cell,
    fs::File,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::{atomic::AtomicBool, Arc, LazyLock, Mutex},
    thread,
    time::Duration,
};
use winit::{
    application::ApplicationHandler, error::EventLoopError, event_loop::EventLoop,
    platform::x11::EventLoopBuilderExtX11, window::WindowAttributes,
};

mod rgba;
mod shimeji;
use shimeji::{BucketError, ShimejiBucket, ShimejiData};

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
    EventLoopError(EventLoopError),
}

#[derive(Debug)]
struct BucketManager {
    /// Shimejis that are waiting
    /// for a context / window to be sent to a bucket.
    pending_shimejis: Vec<ShimejiConfig>,
    buckets: Vec<ShimejiBucket>,
    should_exit: Arc<AtomicBool>,
}

const WINDOW_ATTRIBS: LazyLock<WindowAttributes> = std::sync::LazyLock::new(|| {
    WindowAttributes::default()
        .with_visible(true)
        .with_transparent(true)
        .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
});

impl ApplicationHandler for BucketManager {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        log::debug!("Resumed");

        self.address_pending_shimejis(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::WindowEvent::*;
        // if self.should_exit.load(std::sync::atomic::Ordering::Acquire) {
        //     event_loop.exit()
        // }
        match event {
            RedrawRequested => {}
            _ => {}
        }
    }
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, _event: ()) {}
}

impl BucketManager {
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
            pending_shimejis: vec![],
            should_exit,
            buckets,
        }
    }
    pub fn add_shimeji(&mut self, conf: ShimejiConfig) {
        self.pending_shimejis.push(conf)
    }
    pub fn run(mut self, mut tray_handle: tray_item::TrayItem) -> Result<(), ManagerError> {
        let copy = Arc::clone(&self.should_exit);
        tray_handle
            .add_menu_item("Kill", move || {
                copy.store(true, std::sync::atomic::Ordering::SeqCst);
            })
            .unwrap();
        let event_loop = EventLoop::builder().with_x11().build().unwrap();
        event_loop.run_app(&mut self)?;
        log::debug!("Manager returned");
        Ok(())
    }

    fn address_pending_shimejis(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // If we don't collect here, the compiler
        // believes a reference is still in use
        let mut buckets_by_count = self
            .buckets
            .iter()
            .sorted_by_key(|x| x.contained_shimejis())
            .enumerate()
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>()
            .into_iter()
            .cycle();

        let mut buckets: Vec<_> = self
            .buckets
            .iter_mut()
            .sorted_by_key(|x| x.contained_shimejis())
            .collect();

        while let Some(pending_shimeji) = self.pending_shimejis.pop() {
            let index = buckets_by_count.next().unwrap();
            let window = event_loop
                .create_window(WINDOW_ATTRIBS.clone())
                .expect("should be able to create window for shimeji");

            buckets[index]
                .deref_mut()
                .add(ShimejiData::with_config(&pending_shimeji), window)
                .expect("should be able to add shimeji to bucket");
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShimejiConfig {
    name: String,
}
fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .env()
        .init()
        .expect("Should be able to set up logger");
    log::debug!("Starting");

    let parallelism = thread::available_parallelism()
        .context("Failed to get available parallelism for this system")?
        .get();
    log::debug!("Available parallelism: {}", parallelism);

    let path = std::option_env!("HOME").unwrap_or("/home/lucy").to_owned() + "/tray_icon-red.png";
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
    log::debug!("Running manager");
    let mut manager = BucketManager::new(parallelism);

    let example_config = ShimejiConfig {
        name: String::from("Name"),
    };

    for _ in 0..50 {
        manager.add_shimeji(example_config.clone());
    }
    manager.add_shimeji(example_config);
    manager.run(tray_handle)?;
    log::debug!("At the end");
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

#[cfg(test)]
mod tests {
    use super::*;
    fn init_logger() {
        // It is non-essential if the logger fails, which can often happen
        // since Rust by default runs threads in parallel.
        // SimpleLogger errors if it was already initialized.
        simple_logger::SimpleLogger::new()
            .with_level(log::LevelFilter::Trace)
            .env()
            .init()
            .ok();
    }
    #[test]
    #[should_panic]
    fn panics_on_amount_0() {
        let _ = BucketManager::new(0);
    }

    #[test]
    fn buckets_are_created_successfully() {
        init_logger();
        let manager = BucketManager::new(1);

        assert!(manager.buckets.first().is_some());
    }

    #[test]
    fn buckets_receive_shimeji_sequentially() -> anyhow::Result<()> {
        init_logger();
        let mut manager = BucketManager::new(1);

        manager.add_shimeji(ShimejiConfig {
            name: String::from("example"),
        });

        assert_eq!(manager.buckets.first().unwrap().contained_shimejis(), 1);
        Ok(())
    }
}
