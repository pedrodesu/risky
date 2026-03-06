//! SBI (Supervisor Binary Interface) wrappers used by the kernel.
//!
//! This module exposes kernel-facing wrappers for supervisor binary interface
//! calls.

use core::arch::asm;

const EID_HSM: usize = 0x48534D;
const EID_TIME: usize = 0x54494D45;
const EID_SPI: usize = 0x735049;

const EID_CONSOLE_PUTCHAR: usize = 0x01;
const EID_CONSOLE_GETCHAR: usize = 0x02;

const HSM_FID_HART_START: usize = 0;
const HSM_FID_HART_SUSPEND: usize = 3;
const TIME_FID_SET_TIMER: usize = 0;
const SPI_FID_SEND_IPI: usize = 0;

#[inline(always)]
fn call(extension: usize, function: usize, arg0: usize, arg1: usize, arg2: usize)
-> (usize, usize)
{
    let error: usize;
    let value: usize;
    unsafe {
        asm!(
            "ecall",
            in("a7") extension,
            in("a6") function,
            in("a0") arg0,
            in("a1") arg1,
            in("a2") arg2,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    (error, value)
}

#[inline]
pub fn hart_start(hart_id: usize, start_addr: usize, opaque: usize) -> bool
{
    let (error, _) = call(EID_HSM, HSM_FID_HART_START, hart_id, start_addr, opaque);
    error == 0
}

#[inline]
pub fn hart_suspend() -> bool
{
    let suspend_type = 0x0;

    let (error, _) = call(EID_HSM, HSM_FID_HART_SUSPEND, suspend_type, 0, 0);
    (error as isize) == 0
}

#[inline]
pub fn set_timer(time: u64)
{
    #[cfg(target_arch = "riscv32")]
    call(
        EID_TIME,
        TIME_FID_SET_TIMER,
        time as usize,
        (time >> 32) as usize,
        0,
    );
    #[cfg(target_arch = "riscv64")]
    call(EID_TIME, TIME_FID_SET_TIMER, time as usize, 0, 0);
}

#[inline]
pub fn send_ipi(hart_mask: usize)
{
    call(EID_SPI, SPI_FID_SEND_IPI, hart_mask, 0, 0);
}

#[inline]
pub fn console_putchar(c: usize)
{
    call(EID_CONSOLE_PUTCHAR, 0, c, 0, 0);
}

#[inline]
pub fn console_getchar() -> isize
{
    let (val, _) = call(EID_CONSOLE_GETCHAR, 0, 0, 0, 0);
    val as isize
}
