use pixels::Pixels;
use std::{
    collections::HashMap,
    num::NonZeroU32,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    thread::{self},
    time::{Duration, Instant},
};
use winit::{
    dpi::{LogicalSize, PhysicalPosition},
    raw_window_handle::HasWindowHandle,
    window::Window,
};

use crate::{bucket::BucketThreadMessage, loader::AnimationData};
use BucketThreadMessage::*;
/// All associated functions run on the inner thread.
///
/// ShimejiWindow is only used in the worker function passed to the spawned thread.
struct ShimejiWindow<'a> {
    window: Arc<Window>,
    pixels: Pixels<'a>,
    data: Arc<ShimejiData>,
    last_rendered_frame: Instant,
    current_frame: Option<NonZeroU32>,
}

impl<'a> ShimejiWindow<'a> {
    pub fn new(arc_window: Arc<Window>, mut pixels: Pixels<'a>, data: Arc<ShimejiData>) -> Self {
        let shimeji_width = data.width;
        let shimeji_height = data.height;
        let _ = arc_window.request_inner_size(LogicalSize::new(shimeji_width, shimeji_height));
        arc_window.set_visible(true);
        pixels.clear_color(pixels::wgpu::Color::TRANSPARENT);

        Self {
            window: arc_window,
            last_rendered_frame: Instant::now(),
            data,
            pixels,
            current_frame: None,
        }
    }
}

impl ShimejiWindow<'_> {
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
pub fn loop_for_shimeji_execution(
    receiver: Receiver<BucketThreadMessage>,
    should_exit: Arc<AtomicBool>,
) -> () {
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
            Add(window, pixels, data) => {
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
                inner_vec.push(ShimejiWindow::new(window, pixels, data))
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
                    Add(window, pixels, data) => {
                        log::debug!("Received window: {0:?}", &window);
                        inner_vec.push(ShimejiWindow::new(window, pixels, data))
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

#[derive(Debug, Clone)]
pub struct ShimejiData {
    pub name: Arc<str>,
    pub height: u32,
    pub width: u32,
    pub animations: HashMap<String, AnimationData>,
}
