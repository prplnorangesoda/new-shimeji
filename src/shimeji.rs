use derive_more::derive::{Display, Error, From};
use softbuffer::{Context, Surface};
use std::{
    cell::Cell,
    num::NonZeroU32,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::x11::EventLoopBuilderExtX11,
    window::{Window, WindowId},
};

use super::rgba::Rgba;
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

impl PartialEq for ShimejiBucket {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ShimejiBucket {}

#[derive(Debug)]
pub enum BucketThreadMessage {
    Add(Window, ShimejiData),
    Remove(WindowId, ShimejiData),
}

use BucketThreadMessage::*;

use crate::ShimejiConfig;

impl Drop for ShimejiBucket {
    fn drop(&mut self) {
        log::debug!("Dropping bucket id {}", self.id);
        self.should_exit.store(true, Ordering::Release);
        self.join_thread().ok();
    }
}

struct ShimejiWindow {
    window: Rc<Window>,
    context: Context<Rc<Window>>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    data: ShimejiData,
    last_rendered_frame: Cell<Instant>,
}

impl ShimejiWindow {
    pub fn new(window: Window, data: ShimejiData) -> Self {
        let rc = Rc::new(window);
        let context = Context::new(Rc::clone(&rc)).unwrap();
        Self {
            window: Rc::clone(&rc),
            surface: Surface::new(&context, Rc::clone(&rc)).unwrap(),
            context,
            last_rendered_frame: Cell::new(Instant::now()),
            data,
        }
    }
    pub fn update(&mut self) {
        // self.event_loop.pump_app_events(Some(Duration::ZERO), self);
        let (width, height) = {
            let size = self.window.inner_size();
            (size.width, size.height)
        };
        self.surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .unwrap();

        let mut buffer = self.surface.buffer_mut().unwrap();
        // println!("Buffer length: {}", buffer.len());
        // println!(
        //     "First four buffer bytes: {:b} {:b} {:b} {:b}",
        //     buffer[0], buffer[1], buffer[2], buffer[3]
        // );
        let color_u32 = Rgba::new(0, 50, 0, 10).to_softbuf_u32();
        buffer.fill(color_u32);
        buffer.present().unwrap();
    }
}

/// The thread is started, we are executing.
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
            Add(window, data) => {
                log::debug!(
                    "Received initial window: {0:?}, data: {1:?}",
                    &window,
                    &data
                );
                inner_vec.push(ShimejiWindow::new(window, data))
            }
            _ => unimplemented!(),
        };
        'has_window: loop {
            log::debug!("Looping 'has_window");
            if should_exit.load(Ordering::Relaxed) {
                break;
            }
            // add a new shimeji, if we're waiting to receive one
            let val = match receiver.try_recv() {
                Err(mpsc::TryRecvError::Empty) => None,
                Err(_) => break,
                Ok(val) => Some(val),
            };

            if let Some(val) = val {
                match val {
                    Add(window, data) => {
                        log::debug!("Received window: {0:?}, data: {1:?}", &window, &data);
                        inner_vec.push(ShimejiWindow::new(window, data))
                    }
                    Remove(..) => todo!(),
                }
            }
            if inner_vec.is_empty() {
                break 'has_window;
            }
            for shimeji in inner_vec.iter_mut() {
                shimeji.update();
                thread::sleep(Duration::from_millis(100))
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
    pub fn init(&mut self) -> Result<(), BucketError> {
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
        if !self.is_running || self.thread.is_none() {
            return Ok(());
        }
        {
            self.sender.take();
            // drop sender, ensuring any in progress recvs are stopped
        }
        match self.thread.take().unwrap().join() {
            Ok(_) => (),
            Err(huh) => log::error!("THREAD JOIN ERROR: {huh:?}"),
        };
        self.is_running = false;
        Ok(())
    }
    ///
    /// # Errors
    /// Errors if `!self.is_running` or if `self.sender` == `None`.
    pub fn add(&mut self, shimeji: ShimejiData, window: Window) -> Result<(), BucketError> {
        if !self.is_running {
            return Err(BucketError::NotRunning);
        }
        self.currently_responsible_shimejis += 1;
        let sender = self.sender.as_ref().ok_or(BucketError::NotRunning)?;
        sender
            .send(BucketThreadMessage::Add(window, shimeji))
            .unwrap();
        Ok(())
    }
    pub fn contained_shimejis(&self) -> usize {
        self.currently_responsible_shimejis
    }
}

#[derive(Debug, Clone)]
pub struct ShimejiData {}
