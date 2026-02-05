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
