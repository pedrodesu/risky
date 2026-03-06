//! 16550-compatible UART driver and console formatting helpers.
//!
//! This module implements the UART backend used for kernel console output.

use core::{
    arch::asm,
    fmt::{self, Write},
};

use spin::{Mutex, Once};

use crate::{
    interrupt,
    mmio::{AccessStrategy, IoWidth, Register},
    soc::uart::*,
};

pub struct Uart
{
    base: usize,
    shift: u8,
    width: IoWidth,
}

impl Uart
{
    #[inline]
    pub const fn with_info((base, shift, width): (usize, u8, IoWidth)) -> Self
    {
        Self { base, shift, width }
    }

    #[inline]
    const fn reg(&self, offset: usize) -> Register<u8>
    {
        Register::new(
            (self.base + (offset << self.shift)) as _,
            AccessStrategy::Shifted { width: self.width },
        )
    }

    pub fn putc(&self, c: u8)
    {
        if c == b'\n'
        {
            self.putc_raw(b'\r');
        }
        self.putc_raw(c);
    }

    pub fn putc_raw(&self, c: u8)
    {
        // We must wait for the UART to be ready to accept a new byte, else we might
        // overwrite a character that hasn't been sent yet (FIFO overflow).
        while (self.reg(lsr::ADDR_OFFSET).read() & lsr::TX_IDLE) == 0
        {
            core::hint::spin_loop();
        }

        // All previous Writes must finish before this Output operation
        unsafe { asm!("fence w, o") }

        self.reg(THR_OFFSET).write(c)
    }
}

impl fmt::Write for Uart
{
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result
    {
        s.bytes().for_each(|c| self.putc(c));
        Ok(())
    }
}

pub static UART: Once<Mutex<Uart>> = Once::new();

mod buffering
{
    use core::{
        fmt::{self, Write},
        sync::atomic::{AtomicBool, Ordering},
    };

    use spin::Mutex;

    use super::Uart;

    const TX_BUF_CAP: usize = 4096;

    static TX_BUFFER: Mutex<TxRing> = Mutex::new(TxRing::new());

    // When enabled, writes bypass the ring buffer after first flushing any
    // buffered bytes. This is used by paths that must force immediate output.
    static DIRECT_MODE: AtomicBool = AtomicBool::new(false);

    struct TxRing
    {
        buf: [u8; TX_BUF_CAP],
        head: usize,
        len: usize,
    }

    impl TxRing
    {
        #[inline]
        const fn new() -> Self
        {
            Self {
                buf: [0; TX_BUF_CAP],
                head: 0,
                len: 0,
            }
        }

        #[inline]
        fn is_full(&self) -> bool
        {
            self.len == TX_BUF_CAP
        }

        fn push(&mut self, byte: u8) -> bool
        {
            if self.is_full()
            {
                return false;
            }

            let tail = (self.head + self.len) % TX_BUF_CAP;
            self.buf[tail] = byte;
            self.len += 1;
            true
        }

        fn pop(&mut self) -> Option<u8>
        {
            if self.len == 0
            {
                return None;
            }

            let byte = self.buf[self.head];
            self.head = (self.head + 1) % TX_BUF_CAP;
            self.len -= 1;
            Some(byte)
        }
    }

    pub fn drain_into(uart: &Uart)
    {
        let mut tx = TX_BUFFER.lock();
        while let Some(byte) = tx.pop()
        {
            uart.putc(byte);
        }
    }

    #[inline]
    pub fn set_direct_mode(enabled: bool)
    {
        DIRECT_MODE.store(enabled, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_direct_mode() -> bool
    {
        DIRECT_MODE.load(Ordering::Relaxed)
    }

    pub fn write(args: fmt::Arguments, uart: Option<&mut Uart>)
    {
        let mut tx = TX_BUFFER.lock();
        let mut writer = BufferedWriter {
            tx: &mut tx,
            uart,
            direct_fallback: false,
        };
        let _ = writer.write_fmt(args);
    }

    struct BufferedWriter<'a>
    {
        tx: &'a mut TxRing,
        uart: Option<&'a mut Uart>,
        direct_fallback: bool,
    }

    impl Write for BufferedWriter<'_>
    {
        fn write_str(&mut self, s: &str) -> fmt::Result
        {
            for byte in s.bytes()
            {
                if self.direct_fallback
                {
                    if let Some(uart) = self.uart.as_deref_mut()
                    {
                        uart.putc(byte);
                        continue;
                    }
                    return Err(fmt::Error);
                }

                if !self.tx.push(byte)
                {
                    let Some(uart) = self.uart.as_deref_mut()
                    else
                    {
                        return Err(fmt::Error);
                    };

                    // Drain the backlog and switch to direct output for the rest
                    // of this message so bytes are not silently dropped.
                    while let Some(pending) = self.tx.pop()
                    {
                        uart.putc(pending);
                    }

                    self.direct_fallback = true;
                    uart.putc(byte);
                }
            }
            Ok(())
        }
    }
}

#[inline]
pub fn drain()
{
    interrupt::with_disabled(|| {
        let Some(uart_mutex) = UART.get()
        else
        {
            return;
        };

        let uart = uart_mutex.lock();
        buffering::drain_into(&uart);
    });
}

#[inline]
pub fn set_direct_mode(enabled: bool)
{
    buffering::set_direct_mode(enabled);
}

#[doc(hidden)]
#[inline]
pub fn _print(args: fmt::Arguments)
{
    // Keep writes atomic with respect to local interrupt handlers.
    interrupt::with_disabled(|| {
        if buffering::is_direct_mode()
        {
            if let Some(uart_mutex) = UART.get()
            {
                let mut uart = uart_mutex.lock();
                buffering::drain_into(&uart);
                uart.write_fmt(args).unwrap();
            }
            return;
        }

        let mut uart_guard = UART.get().map(|m| m.lock());
        buffering::write(args, uart_guard.as_deref_mut());
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::drivers::uart::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}
