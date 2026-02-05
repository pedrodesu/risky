use core::arch::asm;

// Standard SBI Extension IDs
const EID_HSM: usize = 0x48534D; // Hart State Management
const EID_TIME: usize = 0x54494D45; // Timer Extension
const EID_SPI: usize = 0x735049; // Supervisor-level IPI Extension

// Legacy SBI Extension IDs
const EID_CONSOLE_PUTCHAR: usize = 0x01;
const EID_CONSOLE_GETCHAR: usize = 0x02;

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
    let function_id: usize = 0;

    let (error, _) = call(EID_HSM, function_id, hart_id, start_addr, opaque);
    error == 0
}

#[inline]
pub fn hart_suspend()
{
    let function_id = 3;
    let suspend_type = 0x0;

    call(EID_HSM, function_id, suspend_type, 0, 0);
}

/// FID: 0 (set_timer)
#[inline]
pub fn set_timer(time: u64)
{
    // No error code, always succeeds
    #[cfg(target_arch = "riscv32")]
    call(EID_TIME, 0, time as usize, (time >> 32) as usize, 0);
    #[cfg(target_arch = "riscv64")]
    call(EID_TIME, 0, time as usize, 0, 0);
}

/// FID: 0 (send_ipi)
#[inline]
pub fn send_ipi(hart_mask: usize)
{
    // No error code, always succeeds
    call(EID_SPI, 0, hart_mask, 0, 0);
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
