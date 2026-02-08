//! This module implements a driver for the 16550A UART (Universal Asynchronous
//! Receiver/Transmitter). It provides functions for initializing the UART,
//! sending (`putc`), and receiving (`get_char`) bytes. It also implements the
//! `fmt::Write` trait, allowing it to be used by the `print!` and `println!`
//! macros.

use core::fmt::{self, Write};

use spin::Mutex;

use crate::sbi;

#[inline]
pub fn get_char() -> Option<u8>
{
    match sbi::console_getchar()
    {
        // -1 means no character is available.
        -1 => None,
        // Otherwise, we have a valid character.
        c => Some(c as u8),
    }
}

#[inline]
pub fn putc(c: u8)
{
    sbi::console_putchar(c as usize);
}

pub struct Uart;

impl fmt::Write for Uart
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result
    {
        s.bytes().for_each(putc);
        Ok(())
    }
}

static UART: Mutex<Uart> = Mutex::new(Uart);

/// A global helper to use formatting without creating a new struct every time.
#[doc(hidden)]
#[inline]
pub fn _print(args: fmt::Arguments)
{
    let mut guard = UART.lock();
    guard.write_fmt(args).unwrap();
}
