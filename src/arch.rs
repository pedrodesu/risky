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

/// Atomically clear bits in a CSR (no register). More efficient for smaller
/// operands.
#[macro_export]
macro_rules! csr_clear_i {
    ($csr:expr, $mask:expr) => {
        core::arch::asm!(concat!("csrci ", $csr, ", {0}"), const $mask)
    };
}

/// Atomically clear bits in a CSR
#[macro_export]
macro_rules! csr_clear {
    ($csr:expr, $mask:expr) => (core::arch::asm!(concat!("csrrc x0, ", $csr, ", {0}"), in(reg) $mask));
}

/// Atomically set bits in a CSR (no register). More efficient for smaller
/// operands.
#[macro_export]
macro_rules! csr_set_i {
    ($csr:expr, $mask:expr) => {
        core::arch::asm!(concat!("csrsi ", $csr, ", {0}"), const $mask)
    };
}

/// Atomically set bits in a CSR
#[macro_export]
macro_rules! csr_set {
    ($csr:expr, $mask:expr) => (core::arch::asm!(concat!("csrrs x0, ", $csr, ", {0}"), in(reg) $mask));
}

/// Read a CSR into a usize
#[macro_export]
macro_rules! csr_read {
    ($csr:expr) => {{
        let r: usize;
        core::arch::asm!(concat!("csrr {0}, ", $csr), out(reg) r);
        r
    }};
}

/// Write an immediate value (0-31) to a CSR
#[macro_export]
macro_rules! csr_write_i {
    ($csr:expr, $val:expr) => (core::arch::asm!(concat!("csrwi ", $csr, ", {0}"), const $val));
}

/// Write a value to a CSR
#[macro_export]
macro_rules! csr_write {
    ($csr:expr, $val:expr) => (core::arch::asm!(concat!("csrw ", $csr, ", {0}"), in(reg) $val));
}

#[inline]
pub fn hart_id() -> usize
{
    let id: usize;
    unsafe { core::arch::asm!("mv {0}, tp", out(reg) id) }
    id
}
