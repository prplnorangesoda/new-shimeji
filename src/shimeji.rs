use derive_more::derive::{Display, Error, From};
use std::{
    cell::Cell,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Instant,
};
use winit::{event_loop::EventLoop, platform::x11::EventLoopBuilderExtX11, window::Window};

#[derive(Debug, Error, Display, From)]
pub enum BucketError {
    DoubleInit,
    NotRunning,
    Io(std::io::Error),
}

/// A bucket of Shimejis, for one thread.
///
/// It will manage its own thread, and have a set of shimejis
/// that its thread should manage.
/// Note that it will live on the main thread, but it maintains a
/// channel to send messages to its inner contained thread
#[derive(Debug)]
pub struct ShimejiBucket {
    pub id: usize,
    is_running: bool,
    thread: Option<JoinHandle<()>>,
    should_exit: Arc<AtomicBool>,
    currently_responsible_shimejis: usize,
    sender: Option<Sender<BucketThreadMessage>>,
}

pub enum BucketThreadMessage {
    Add(ShimejiData),
    Remove(ShimejiData),
}

use BucketThreadMessage::*;

use crate::ShimejiConfig;

impl Drop for ShimejiBucket {
    fn drop(&mut self) {
        self.should_exit.store(true, Ordering::Release);
        self.join_thread().ok();
    }
}

struct ShimejiWindow {
    window: Option<Window>,
    event_loop: EventLoop<()>,
    data: ShimejiData,
    last_rendered_frame: Cell<Instant>,
}

impl ShimejiWindow {
    pub fn new(data: ShimejiData) -> Self {
        let event_loop = EventLoop::builder().with_x11().build().unwrap();
        Self {
            event_loop,
            window: None,
            last_rendered_frame: Cell::new(Instant::now()),
            data,
        }
    }
    pub fn update(&mut self) {
        // self.event_loop.pump_app_events(Some(Duration::ZERO), self);
        unimplemented!()
    }
}

#[inline]
fn loop_for_shimeji_execution(
    receiver: Receiver<BucketThreadMessage>,
    should_exit: Arc<AtomicBool>,
) {
    while !should_exit.load(Ordering::Relaxed) {
        let mut inner_vec = vec![];
        match receiver.recv().expect(
            "should be able to receive value, else sender hung up without sending single shimeji",
        ) {
            Add(val) => {
                log::debug!("Received shimeji: {val:?}");
            }
            _ => unimplemented!(),
        };
        'has_shimeji: loop {
            if should_exit.load(Ordering::Relaxed) {
                break;
            }
            // add a new shimeji, if we're waiting to receive one
            let val = match receiver.try_recv() {
                Err(mpsc::TryRecvError::Empty) => None,
                Err(_) => break,
                Ok(val) => Some(val),
            };

            if val.is_some() {
                match val.unwrap() {
                    Add(new_shimeji) => inner_vec.push(ShimejiWindow::new(new_shimeji)),
                    Remove(_) => todo!(),
                }
            }
            if inner_vec.is_empty() {
                break 'has_shimeji;
            }
            for shimeji in inner_vec.iter_mut() {
                shimeji.update();
            }
        }
    }
}
impl ShimejiBucket {
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    pub fn new(id: usize, should_exit: Arc<AtomicBool>) -> Self {
        ShimejiBucket {
            id,
            is_running: false,
            thread: None,
            should_exit,
            currently_responsible_shimejis: 0,
            sender: None,
        }
    }
    pub fn init<'a>(&mut self) -> Result<(), BucketError> {
        if self.is_running {
            return Err(BucketError::DoubleInit);
        }
        let should_exit = self.should_exit.clone();
        let id = self.id;
        log::trace!("Initting bucket id: {id}");
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("Bucket thread {}", &id))
            .spawn(move || {
                loop_for_shimeji_execution(receiver, should_exit);
            })?;
        self.sender = Some(sender.clone());
        self.thread = Some(thread);
        self.is_running = true;
        Ok(())
    }
    pub fn join_thread(&mut self) -> Result<(), BucketError> {
        if self.thread.is_none() {
            return Ok(());
        }
        match self.thread.take().unwrap().join() {
            Ok(_) => (),
            Err(huh) => println!("THREAD JOIN ERROR: {huh:?}"),
        };
        Ok(())
    }
    ///
    /// # Errors
    /// Errors if `!self.is_running` or if `self.sender` == `None`.
    pub fn add(&mut self, shimeji: ShimejiData) -> Result<(), BucketError> {
        if !self.is_running {
            return Err(BucketError::NotRunning);
        }
        self.currently_responsible_shimejis += 1;
        let sender = self.sender.as_ref().ok_or(BucketError::NotRunning)?;
        sender.send(BucketThreadMessage::Add(shimeji)).unwrap();
        Ok(())
    }
    pub fn contained_shimejis(&self) -> usize {
        self.currently_responsible_shimejis
    }
}

#[derive(Debug)]
pub struct ShimejiData {
    name: String,
}

impl ShimejiData {
    pub fn with_config(config: &ShimejiConfig) -> Self {
        Self {
            name: config.name.clone(),
        }
    }
}
