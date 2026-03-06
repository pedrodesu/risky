//! Global kernel logger implementation.
//!
//! This module configures the kernel logging backend and formatting.

use owo_colors::OwoColorize;

struct Logger;

impl log::Log for Logger
{
    #[inline]
    fn enabled(&self, _: &log::Metadata) -> bool
    {
        true
    }

    #[inline]
    fn log(&self, record: &log::Record)
    {
        let level_str = match record.level()
        {
            log::Level::Error => "ERROR".red().into_styled(),
            log::Level::Warn => "WARN".yellow().into_styled(),
            log::Level::Info => "INFO".green().into_styled(),
            log::Level::Debug => "DEBUG".blue().into_styled(),
            log::Level::Trace => "TRACE".purple().into_styled(),
        };

        println!(
            "[{}] ({}) {}",
            level_str,
            record.module_path().unwrap_or("unknown"),
            record.args()
        );
    }

    #[inline]
    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

#[inline]
pub fn init()
{
    log::set_logger(&LOGGER).expect("Failed to initialize logger");
    log::set_max_level(log::LevelFilter::max());
}
