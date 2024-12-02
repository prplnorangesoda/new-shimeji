use std::{
    any::Any,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle, Thread},
    time::Duration,
};

use derive_more::derive::{Display, Error, From};

#[derive(Debug, Error, Display, From)]
pub enum BucketError {
    Io(std::io::Error),
}

/// A bucket of Shimejis, for one thread.
///
/// It will manage its own thread, and have a set of shimejis
/// that its thread should manage.
/// Note that it will live on the main thread, but it maintains a
/// channel to send messages to its inner contained thread
#[derive(Debug)]
pub struct ShimejiBucket<'a> {
    pub id: usize,
    thread: Option<JoinHandle<()>>,
    should_exit: Arc<AtomicBool>,
    currently_responsible_shimejis: Vec<Shimeji<'a>>,
}

impl ShimejiBucket<'_> {
    pub fn new(id: usize, should_exit: Arc<AtomicBool>) -> Self {
        ShimejiBucket {
            id,
            thread: None,
            should_exit,
            currently_responsible_shimejis: vec![],
        }
    }
    pub fn init(&mut self) -> Result<(), BucketError> {
        let should_exit = self.should_exit.clone();
        let id = self.id;
        self.thread = Some(
            thread::Builder::new()
                .name(format!("Bucket thread {}", &id))
                .spawn(move || loop {
                    if should_exit.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(Duration::from_secs(0));
                    log::info!("HI FROM THREAD {}", id);
                })?,
        );
        Ok(())
    }
    pub fn join_thread(self) -> Result<(), BucketError> {
        if self.thread.is_none() {
            return Ok(());
        }
        self.thread.unwrap().join().unwrap();
        Ok(())
    }
    pub fn add(&mut self) {
        todo!()
    }
    pub fn len(&self) -> usize {
        self.currently_responsible_shimejis.len()
    }
}

#[derive(Debug)]
pub struct Shimeji<'a> {
    name: &'a str,
}
