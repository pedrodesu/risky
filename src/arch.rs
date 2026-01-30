use core::arch::asm;

pub mod cause
{
    pub mod interrupts
    {
        pub const MACHINE_SOFTWARE_INTERRUPT: usize = 3; // Software interrupt (Multicore communication (Scheduler))
        pub const MACHINE_TIMER_INTERRUPT: usize = 7; // Timer (CLINT in our case)
        pub const MACHINE_EXTERNAL_INTERRUPT: usize = 11; // External communication (UART)
    }

    pub mod exceptions
    {
        pub const INSTRUCTION_ACCESS_FAULT: usize = 1;
        pub const ILLEGAL_INSTRUCTION: usize = 2;
        pub const LOAD_ACCESS_FAULT: usize = 5;
        pub const STORE_ACCESS_FAULT: usize = 7;

        pub const USER_ECALL: usize = 8;
        pub const SUPERVISOR_ECALL: usize = 9;
        pub const MACHINE_ECALL: usize = 11;
    }
}

/// Atomically set bits in a CSR immediately (no register). More efficient for
/// smaller operands.
#[macro_export]
macro_rules! csr_set_i {
    ($csr:expr, $mask:expr) => (asm!(concat!("csrsi ", $csr, ", {0}"), const $mask));
}

/// Atomically set bits in a CSR
#[macro_export]
macro_rules! csr_set {
    ($csr:expr, $mask:expr) => (asm!(concat!("csrrs x0, ", $csr, ", {0}"), in(reg) $mask));
}

/// Read a CSR into a usize
#[macro_export]
macro_rules! csr_read {
    ($csr:expr) => {{
        let r: usize;
        asm!(concat!("csrr {0}, ", $csr), out(reg) r);
        r
    }};
}

/// Write a value to a CSR
#[macro_export]
macro_rules! csr_write {
    ($csr:expr, $val:expr) => (asm!(concat!("csrw ", $csr, ", {0}"), in(reg) $val));
}

#[inline]
pub fn hart_id() -> usize
{
    unsafe { csr_read!("mhartid") }
}
