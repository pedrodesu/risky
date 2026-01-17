//! Defines the global `print!` and `println!` macros for writing formatted
//! text to the UART driver. This provides a familiar, convenient interface for
//! kernel-level logging and output.

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::uart::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
