// Registers for Hart 0 M-Mode
pub mod plic
{
    pub const BASE: usize = 0x0c00_0000;
    pub const PRIORITY_BASE: usize = BASE;
    pub const ENABLE_BASE: usize = BASE + 0x2000;
    pub const THRESHOLD_BASE: usize = BASE + 0x200000;
    pub const CLAIM_BASE: usize = BASE + 0x200004;
}

/// Core Local Interruptor (CLINT) - Machine-mode Timer Registers
pub mod clint
{
    pub const MTIME: *mut u64 = 0x0200_bff8 as _;
    pub const MTIMECMP: *mut u64 = 0x0200_4000 as _;
}

/// Universal Asynchronous Receiver/Transmitter (UART) constants
pub mod uart
{
    pub const IRQ: u32 = 10;

    pub const BASE: usize = 0x1000_0000;

    pub const RBR: *mut u8 = BASE as _; // Receiver Buffer Register (Read only)
    pub const THR: *mut u8 = BASE as _; // Transmit Holding Register (Write only)
    pub const IER: *mut u8 = (BASE + 1) as _; // Interrupt Enable Register
    pub const LSR: *mut u8 = (BASE + 5) as _; // Line Status Register
    pub const FCR: *mut u8 = (BASE + 2) as _; // FIFO Control Register (Write Only)
}
