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
    sender: Option<Sender<BucketThreadMessage<'static>>>,
}

impl PartialEq for ShimejiBucket {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ShimejiBucket {}

#[derive(Debug)]
pub enum BucketThreadMessage<'a> {
    Add(Arc<Window>, Pixels<'a>, Arc<ShimejiData>),
    Resized {
        id: WindowId,
        size: PhysicalSize<u32>,
    },
    Remove(WindowId),
}

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
};

use anyhow::Context;
use derive_more::derive::{Display, Error, From};
use pixels::{Pixels, PixelsBuilder, SurfaceTexture};
use winit::{
    dpi::PhysicalSize,
    raw_window_handle::HasWindowHandle,
    window::{Window, WindowId},
};

use crate::shimeji::ShimejiData;

impl Drop for ShimejiBucket {
    fn drop(&mut self) {
        log::debug!("Dropping bucket id {}", self.id);
        self.should_exit.store(true, Ordering::Release);
        self.join_thread().ok();
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
        let id = self.id.clone();
        let thread = thread::Builder::new()
            .name(format!("Bucket {} thread", &self.id))
            .spawn(move || {
                crate::shimeji::loop_for_shimeji_execution(receiver, should_exit, id);
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
            Err(why) => log::error!("THREAD JOIN ERROR on id {}: {why:?}", self.id),
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

        let rc = Arc::new(window);
        let pixels = {
            let window_size = rc.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, Arc::clone(&rc));
            PixelsBuilder::new(shimeji.width, shimeji.height, surface_texture)
                .build()
                .unwrap()
        };
        assert!(rc.window_handle().is_ok());
        sender
            .send(BucketThreadMessage::Add(rc, pixels, shimeji))
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
