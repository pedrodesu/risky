use owo_colors::OwoColorize;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::uart::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

struct Logger;

impl log::Log for Logger
{
    #[inline]
    fn enabled(&self, _: &log::Metadata) -> bool
    {
        // metadata.level() <= Level::Info
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
