use anyhow::Context;
use derive_more::derive::{Display, Error, From};
use pixels::{Pixels, SurfaceTexture};
use std::{
    cell::Cell,
    collections::HashMap,
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
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    window::{Window, WindowId},
};

use crate::loader::AnimationData;

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
    Add(Window, Arc<ShimejiData>),
    Resized {
        id: WindowId,
        size: PhysicalSize<u32>,
    },
    Remove(WindowId),
}

use BucketThreadMessage::*;

impl Drop for ShimejiBucket {
    fn drop(&mut self) {
        log::debug!("Dropping bucket id {}", self.id);
        self.should_exit.store(true, Ordering::Release);
        self.join_thread().ok();
    }
}

struct ShimejiWindow<'a> {
    window: Arc<Window>,
    pixels: Pixels<'a>,
    data: Arc<ShimejiData>,
    last_rendered_frame: Instant,
    current_frame: Option<NonZeroU32>,
}

impl ShimejiWindow<'_> {
    pub fn new(window: Window, data: Arc<ShimejiData>) -> Self {
        let shimeji_width = data.width;
        let shimeji_height = data.height;
        let rc = Arc::new(window);
        let mut pixels = {
            let window_size = rc.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, Arc::clone(&rc));
            Pixels::new(shimeji_width, shimeji_height, surface_texture).unwrap()
        };
        let _ = rc.request_inner_size(LogicalSize::new(shimeji_width, shimeji_height));
        rc.set_visible(true);
        pixels.clear_color(pixels::wgpu::Color::TRANSPARENT);

        Self {
            window: rc,
            last_rendered_frame: Instant::now(),
            data,
            pixels,
            current_frame: None,
        }
    }
    pub fn update(&mut self) {
        let idle_animation = self.data.animations.get("idle").unwrap();
        let time_between_frames = Duration::from_secs_f64(1.0 / idle_animation.fps);

        let delta_time = self.last_rendered_frame.elapsed();
        log::trace!("delta_time: {delta_time:?}, time_between_frames: {time_between_frames:?}");
        if delta_time < time_between_frames {
            return;
        } // passed frame cap, time to render
        log::debug!("delta_time check passed");

        let frame_index: usize = self
            .current_frame
            .unwrap_or(unsafe { NonZeroU32::new_unchecked(1) })
            .get()
            .try_into()
            .unwrap();

        let mut frame_index = frame_index + 1;
        self.current_frame = Some(NonZeroU32::new(frame_index.try_into().unwrap()).unwrap());

        let zero_indexed_frame_index = frame_index - 1;
        if idle_animation
            .frames
            .get(zero_indexed_frame_index)
            .is_none()
        {
            self.current_frame = Some(unsafe { NonZeroU32::new_unchecked(1) });
            frame_index = 1;
        }
        log::debug!("frame_index: {frame_index}");

        let zero_indexed_frame_index = frame_index - 1;
        let frame = &idle_animation.frames[zero_indexed_frame_index];
        {
            let buffer = self.pixels.frame_mut();
            for (color, pixel) in frame
                .pixels_row_major
                .iter()
                .zip(buffer.chunks_exact_mut(4))
            {
                let slice = [color.red, color.green, color.blue, color.alpha];
                pixel.copy_from_slice(&slice);
                //     buffer[index] = value.to_softbuf_u32();
            }
        }

        let _ = self.pixels.render();
        if !self.window.is_visible().unwrap() {
            self.window.set_visible(true);
        }
        self.last_rendered_frame = Instant::now();
        // buffer.present().unwrap();
    }
}

/// The thread is started, we are executing.
#[inline]
fn loop_for_shimeji_execution(
    receiver: Receiver<BucketThreadMessage>,
    should_exit: Arc<AtomicBool>,
) {
    'running: while !should_exit.load(Ordering::Relaxed) {
        let mut inner_vec = vec![];
        let recv = receiver.recv();
        let recv = match recv {
            Ok(val) => val,
            Err(_) => {
                log::debug!("Sender hung up without sending any shimeji");
                break 'running;
            }
        };
        match recv {
            Add(window, data) => {
                log::debug!("Received initial window: {0:?}", &window,);
                let monitor = window.current_monitor();
                match monitor {
                    Some(monitor) => {
                        // log::debug!("monitor: {monitor:?}");
                        let size = monitor.size();
                        let position = window.outer_position().unwrap();
                        log::debug!("monitor size: {size:?}");
                        log::debug!("window position: {position:?}");
                        window.set_outer_position(PhysicalPosition::new(
                            0, // size.height - window.inner_size().height,
                            500,
                        ));
                    }
                    None => {
                        log::warn!("Current monitor could not be detected");
                        window.set_outer_position(PhysicalPosition::new(0, 0));
                    }
                }
                inner_vec.push(ShimejiWindow::new(window, data))
            }
            _ => unimplemented!(),
        };
        'has_window: loop {
            log::trace!("Looping 'has_window");
            if should_exit.load(Ordering::Relaxed) {
                log::debug!("Should exit, breaking loop");
                break 'running;
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
                        log::debug!("Received window: {0:?}", &window);
                        inner_vec.push(ShimejiWindow::new(window, data))
                    }
                    Remove(..) => todo!(),
                    Resized { id, size } => {
                        let shimeji = inner_vec
                            .iter_mut()
                            .find(|shimeji| (**shimeji).window.id() == id)
                            .expect("resized ID should be valid");
                        let _ = shimeji.pixels.resize_surface(size.width, size.height);
                    }
                }
            }
            if inner_vec.is_empty() {
                log::debug!("No windows in inner_vec! Stopping 'has_window");
                break 'has_window;
            }
            for shimeji in inner_vec.iter_mut() {
                shimeji.update();
                thread::yield_now();
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
        log::trace!("Initting bucket id: {}", &self.id);
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("Bucket {} thread", &self.id))
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
        // drop sender, ensuring any in progress recvs are stopped
        drop(self.sender.take());
        match self.thread.take().unwrap().join() {
            Ok(_) => log::debug!("Thread joined successfully on id {}", self.id),
            Err(huh) => log::error!("THREAD JOIN ERROR on id {}: {huh:?}", self.id),
        };
        self.is_running = false;
        Ok(())
    }
    ///
    /// # Errors
    /// Errors if `!self.is_running` or if `self.sender` == `None`.
    pub fn add(&mut self, shimeji: Arc<ShimejiData>, window: Window) -> Result<(), BucketError> {
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
    pub fn was_resized(
        &mut self,
        id: WindowId,
        size: PhysicalSize<u32>,
    ) -> Result<(), BucketError> {
        if !self.is_running {
            return Err(BucketError::NotRunning);
        }
        let sender = self.sender.as_ref().ok_or(BucketError::NotRunning)?;
        sender
            .send(BucketThreadMessage::Resized { id, size })
            .context("should be able to send resized message")
            .unwrap();
        Ok(())
    }
    pub fn contained_shimejis(&self) -> usize {
        self.currently_responsible_shimejis
    }
}

#[derive(Debug, Clone)]
pub struct ShimejiData {
    pub name: Arc<str>,
    pub height: u32,
    pub width: u32,
    pub animations: HashMap<String, AnimationData>,
}
