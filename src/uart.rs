//! This module implements a driver for the 16550A UART (Universal Asynchronous
//! Receiver/Transmitter). It provides functions for initializing the UART,
//! sending (`putc`), and receiving (`get_char`) bytes. It also implements the
//! `fmt::Write` trait, allowing it to be used by the `print!` and `println!`
//! macros.

use core::{
    arch::asm,
    fmt::{self, Write},
};

use crate::{soc::uart::*, spin::Mutex};

/// Initialize the UART
/// In many environments (like QEMU), the baud rate is pre-set,
/// but we must ensure interrupts are configured correctly
pub fn init()
{
    // Disable interrupts during setup
    IER.write(0x00);

    // Enable and Reset FIFOs (Bit 0: Enable, Bit 1: Clear RX, Bit 2: Clear TX)
    FCR.write(0b0000_0111);

    // Enable RX interrupts
    IER.write(0x01)
}

#[inline]
pub fn get_char() -> Option<u8>
{
    if (lsr::ADDR.read() & lsr::RX_READY) != 0
    {
        Some(RBR.read())
    }
    else
    {
        None
    }
}

/// Safety: We loop until LSR is idle
pub fn putc(c: u8)
{
    // We must wait for the UART to be ready to accept a new byte, else we might
    // overwrite a character that hasn't been sent yet (FIFO overflow).
    while (lsr::ADDR.read() & lsr::TX_IDLE) == 0
    {
        core::hint::spin_loop();
    }

    // Wait for the other output writes
    unsafe { asm!("fence ow, ow") }

    THR.write(c)
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

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::uart::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
