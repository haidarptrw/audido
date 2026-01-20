pub mod metadata;
pub mod dsp;
pub mod engine;
pub mod source;
pub mod commands;
pub mod app;
pub mod browser;

pub fn init_engine() {

}

pub fn init_logger() {

}

pub fn init() -> anyhow::Result<()> {
    init_logger();
    init_engine();
    Ok(())
}