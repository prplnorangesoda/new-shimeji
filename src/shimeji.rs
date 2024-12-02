use std::{
    any::Any,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc,
    },
    thread::{self, JoinHandle, Thread},
    time::Duration,
};

use anyhow::Context;
use derive_more::derive::{Display, Error, From};

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
    currently_responsible_shimejis: Vec<Shimeji>,
    sender: Option<Sender<BucketThreadMessage>>,
}

pub enum BucketThreadMessage {
    Add(Shimeji),
    Remove(Shimeji),
}

use BucketThreadMessage::*;

use crate::ShimejiConfig;

impl Drop for ShimejiBucket {
    fn drop(&mut self) {
        self.should_exit.store(true, Ordering::Release);
        self.join_thread().ok();
    }
}

impl ShimejiBucket {
    pub fn new(id: usize, should_exit: Arc<AtomicBool>) -> Self {
        ShimejiBucket {
            id,
            is_running: false,
            thread: None,
            should_exit,
            currently_responsible_shimejis: vec![],
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
        let (sender, receiver) = mpsc::channel::<BucketThreadMessage>();
        let thread = thread::Builder::new()
            .name(format!("Bucket thread {}", &id))
            .spawn(move || {
                let receiver = receiver;
                let mut inner_vec = vec![];
                match receiver.recv().expect("should receive succesfully") {
                    Add(val) => {}
                    _ => unimplemented!(),
                };
                loop {
                    if should_exit.load(Ordering::Relaxed) {
                        break;
                    }
                    let val = match receiver.try_recv() {
                        Err(mpsc::TryRecvError::Empty) => None,
                        Err(_) => break,
                        Ok(val) => Some(val),
                    };
                    if val.is_some() {
                        match val.unwrap() {
                            Add(new_shimeji) => inner_vec.push(new_shimeji),
                            Remove(_) => todo!(),
                        }
                    }
                }
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
    pub fn add(&mut self, shimeji: Shimeji) -> Result<(), BucketError> {
        if !self.is_running {
            return Err(BucketError::NotRunning);
        }

        let sender = self.sender.as_ref().ok_or(BucketError::NotRunning)?;
        sender.send(BucketThreadMessage::Add(shimeji)).unwrap();
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.currently_responsible_shimejis.len()
    }
}

#[derive(Debug)]
pub struct Shimeji {
    name: String,
}

impl Shimeji {
    pub fn with_config(config: &ShimejiConfig) -> Self {
        Self {
            name: config.name.clone(),
        }
    }
}
