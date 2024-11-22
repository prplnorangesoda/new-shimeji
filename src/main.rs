#![deny(unused_must_use)]
use anyhow::Context as ContextTrait;
use winit::event_loop::EventLoop;

mod app;

use app::App;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new().unwrap();

    let mut app = App::default();
    event_loop
        .run_app(&mut app)
        .context("Event loop broke running app")?;
    Ok(())
}
