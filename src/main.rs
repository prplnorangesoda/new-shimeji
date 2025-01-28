#![deny(unused_must_use)]
#![allow(dead_code)]

use anyhow::Context as _;
use cfg_if::cfg_if;
use itertools::Itertools;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::OsString,
    ops::Deref,
    rc::Rc,
    sync::{atomic::AtomicBool, Arc, LazyLock},
    thread,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{WindowAttributes, WindowId, WindowLevel},
};

mod loader;
mod rgba;
mod shimeji;
mod xml_parser;
use shimeji::{BucketError, ShimejiBucket, ShimejiData};

use derive_more::{derive::From, Display, Error};

#[derive(Debug)]
enum Status {
    Ok,
    Exiting,
}

impl Status {
    /// Returns `true` if the status is [`Ok`].
    ///
    /// [`Ok`]: Status::Ok
    #[must_use]
    fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }
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
    should_exit: Arc<AtomicBool>,
    /// Shimejis that are waiting
    /// for a context / window to be sent to a bucket.
    pending_shimejis: Vec<Arc<ShimejiData>>,
    buckets: Vec<Rc<RefCell<ShimejiBucket>>>,
    buckets_windows_map: HashMap<WindowId, Rc<RefCell<ShimejiBucket>>>,
}
cfg_if! {
    if #[cfg(target_os = "linux")] {
        use winit::platform::x11::{EventLoopBuilderExtX11, WindowAttributesExtX11, WindowType};
        static WINDOW_ATTRIBS: LazyLock<WindowAttributes> = std::sync::LazyLock::new(|| {
            WindowAttributes::default()
                .with_visible(true)
                .with_transparent(true)
                .with_decorations(false)
                .with_x11_window_type(vec![WindowType::Dock])
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_inner_size(PhysicalSize::new(10, 10))
        });
    } else {
        static WINDOW_ATTRIBS: LazyLock<WindowAttributes> = std::sync::LazyLock::new(|| {
            WindowAttributes::default()
                .with_visible(true)
                .with_transparent(true)
                .with_decorations(false)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_inner_size(PhysicalSize::new(10, 10))
        });
    }

}

impl ApplicationHandler for BucketManager {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Resumed");

        self.address_pending_shimejis(event_loop);
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        log::debug!("Exiting");
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        use WindowEvent::*;
        if self.should_exit.load(std::sync::atomic::Ordering::Acquire) {
            event_loop.exit()
        }
        log::trace!("WindowEvent: {event:?}");
        match event {
            RedrawRequested => {
                log::trace!("WindowEvent: RedrawRequested")
            }
            Resized(size) => {
                log::trace!("WindowEvent: Resized");
                let bucket: &RefCell<ShimejiBucket> =
                    Rc::deref(self.buckets_windows_map.get(&window_id).unwrap());
                bucket
                    .borrow_mut()
                    .was_resized(window_id, size)
                    .context("could not resize window on resize event received")
                    .unwrap();
            }
            MouseInput {
                device_id,
                state,
                button,
            } => {}
            _ => (),
        }
    }
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: ()) {}
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
            buckets.push(Rc::new(RefCell::new(bucket)));
        }
        Self {
            pending_shimejis: vec![],
            should_exit,
            buckets,
            buckets_windows_map: HashMap::new(),
        }
    }
    pub fn add_shimeji(&mut self, pending: Arc<ShimejiData>) {
        self.pending_shimejis.push(pending)
    }
    pub fn run(mut self, tray_handle: Option<tray_item::TrayItem>) -> Result<(), ManagerError> {
        let copy = Arc::clone(&self.should_exit);
        if let Some(mut handle) = tray_handle {
            handle
                .add_menu_item("Kill", move || {
                    copy.store(true, std::sync::atomic::Ordering::SeqCst);
                })
                .unwrap();
        }

        cfg_if! {
            if #[cfg(target_os = "linux")] {
                let event_loop = EventLoop::builder().with_x11().build().unwrap();
            } else {
                let event_loop = EventLoop::new().unwrap();
            }
        }
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
            .sorted_by_key(|x| Rc::deref(x).borrow_mut().contained_shimejis())
            .enumerate()
            .map(|(idx, _)| idx)
            .collect::<Vec<_>>()
            .into_iter()
            .cycle();

        let buckets: Vec<_> = self
            .buckets
            .iter_mut()
            .sorted_by_key(|x| Rc::deref(x).borrow_mut().contained_shimejis())
            .collect();

        // while we still have pending shimejis...
        while let Some(pending_shimeji) = self.pending_shimejis.pop() {
            let index = buckets_by_count.next().unwrap();
            let window = event_loop
                .create_window(WINDOW_ATTRIBS.clone())
                .expect("should be able to create window for shimeji");

            let id = window.id();

            let bucket_rc = &buckets[index];
            let mut bucket_to_add_to = Rc::deref(bucket_rc).borrow_mut();
            bucket_to_add_to
                .add(pending_shimeji, window)
                .expect("should be able to add shimeji to bucket");
            let clone = Rc::clone(bucket_rc);
            self.buckets_windows_map.insert(id, clone);
        }
    }
}
fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()
        .expect("Should be able to set up logger");
    log::debug!("Starting");

    let parallelism = thread::available_parallelism()
        .context("Failed to get available parallelism for this system")?
        .get();
    log::debug!("Available parallelism: {}", parallelism);

    let icon_red = tray_item::IconSource::Resource("/home/lucy/tray_icon-red.png");

    let tray_handle = tray_item::TrayItem::new("Example", icon_red).ok();
    log::debug!("Running manager");
    let mut manager = BucketManager::new(parallelism);
    let file_name =
        std::env::var_os("SHIMEJI_CONFIG_FILE").unwrap_or(OsString::from("./default.xml"));
    let config = loader::create_shimeji_data_from_file_name(file_name)?;
    let config = Arc::new(config);

    for _ in 0..1 {
        manager.add_shimeji(config.clone());
    }
    manager.add_shimeji(config);
    manager.run(tray_handle)?;
    log::debug!("At the end");
    Ok(())
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

    // #[test]
    // fn buckets_receive_shimeji_sequentially() -> anyhow::Result<()> {
    //     init_logger();
    //     let mut manager = BucketManager::new(1);

    //     manager.add_shimeji(ShimejiConfig {
    //         name: String::from("example"),
    //     });

    //     assert_eq!(manager.buckets.first().unwrap().contained_shimejis(), 1);
    //     Ok(())
    // }
}
