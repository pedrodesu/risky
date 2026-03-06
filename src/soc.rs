//! SoC register layout constants.

// Registers for Hart 0 M-Mode
pub mod plic
{
    pub const BASE: usize = 0x0c00_0000;
    pub const THRESHOLD_BASE: usize = BASE + 0x200000;
}

/// Universal Asynchronous Receiver/Transmitter (UART) constants
pub mod uart
{
    pub const IRQ: u32 = 10;

    pub const RBR_OFFSET: usize = 0; // Receiver Buffer Register (Read only)
    pub const THR_OFFSET: usize = 0; // Transmit Holding Register (Write only)
    pub const IER_OFFSET: usize = 1; // Interrupt Enable Register
    pub const FCR_OFFSET: usize = 2; // FIFO Control Register (Write Only)

    pub mod lsr
    {
        pub const ADDR_OFFSET: usize = 5; // Line Status Register

        pub const RX_READY: u8 = 1 << 0; // The Data Ready bit
        pub const TX_IDLE: u8 = 1 << 5; // The Transmit Holding Register Empty bit
    }
}
