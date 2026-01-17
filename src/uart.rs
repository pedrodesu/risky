//! This module implements a driver for the 16550A UART (Universal Asynchronous
//! Receiver/Transmitter). It provides functions for initializing the UART,
//! sending (`putc`), and receiving (`get_char`) bytes. It also implements the
//! `fmt::Write` trait, allowing it to be used by the `print!` and `println!`
//! macros.

use core::{
    arch::asm,
    fmt::{self, Write},
};

use crate::soc::uart::*;

/// Initialize the UART
/// In many environments (like QEMU), the baud rate is pre-set,
/// but we must ensure interrupts are configured correctly
pub unsafe fn init()
{
    unsafe {
        // Disable interrupts during setup
        IER.write_volatile(0x00);

        // Enable and Reset FIFOs (Bit 0: Enable, Bit 1: Clear RX, Bit 2: Clear TX)
        FCR.write_volatile(0b0000_0111);

        // Enable RX interrupts
        enable_rx_interrupt();
    }
}

#[inline]
unsafe fn enable_rx_interrupt()
{
    unsafe { IER.write_volatile(0x01) }
}

/// The Transmit Holding Register Empty bit (Bit 5 of LSR)
const LSR_TX_IDLE: u8 = 1 << 5;
/// The Data Ready bit (Bit 0 of LSR)
const LSR_RX_READY: u8 = 1 << 0;

#[inline]
pub unsafe fn get_char() -> Option<u8>
{
    if (unsafe { LSR.read_volatile() } & LSR_RX_READY) != 0
    {
        Some(unsafe { RBR.read_volatile() })
    }
    else
    {
        None
    }
}

pub unsafe fn putc(c: u8)
{
    // BLOCKING WAIT: We must wait for the UART to be ready to accept a new byte
    // If we don't check LSR bit 5, we might overwrite a character that hasn't been
    // sent yet (FIFO overflow).
    while (unsafe { LSR.read_volatile() } & LSR_TX_IDLE) == 0
    {
        core::hint::spin_loop();
    }

    // Wait for the other output writes
    unsafe { asm!("fence ow, ow") }

    unsafe { THR.write_volatile(c) }
}

pub struct Uart;

impl fmt::Write for Uart
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result
    {
        for b in s.bytes()
        {
            unsafe { putc(b) };
        }
        Ok(())
    }
}

/// A global helper to use formatting without creating a new struct every time.
#[doc(hidden)]
#[inline]
pub fn _print(args: fmt::Arguments)
{
    let _ = Uart.write_fmt(args);
}
