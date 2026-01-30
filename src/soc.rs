use core::ptr::NonNull;

/// Register must not be null!
pub struct Register<T>
{
    ptr: NonNull<T>,
}

impl<T> From<*mut T> for Register<T>
{
    #[inline]
    fn from(addr: *mut T) -> Self
    {
        Self {
            ptr: unsafe { NonNull::new_unchecked(addr as _) },
        }
    }
}

impl<T> Register<T>
{
    pub const fn new(addr: *mut T) -> Self
    {
        assert!(!addr.is_null());

        Self {
            ptr: unsafe { NonNull::new_unchecked(addr as _) },
        }
    }

    pub fn read(&self) -> T
    {
        unsafe { self.ptr.read_volatile() }
    }

    pub fn write(&self, value: T)
    {
        unsafe { self.ptr.write_volatile(value) }
    }
}

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
    use crate::soc::Register;

    pub const BASE: usize = 0x0200_0000;

    #[cfg(target_arch = "riscv64")]
    pub const MTIME: Register<u64> = Register::new((BASE + 0xBFF8) as _);

    #[cfg(target_arch = "riscv32")]
    pub const MTIME: Register<u32> = Register::new((BASE + 0xBFF8) as _);
    #[cfg(target_arch = "riscv32")]
    pub const MTIMEH: Register<u32> = Register::new((BASE + 0xBFFC) as _);

    pub const MTIMECMP_BASE: usize = BASE + 0x4000;

    #[inline]
    pub const fn mtimecmp(hart_id: usize) -> Register<u64>
    {
        Register::new((MTIMECMP_BASE + (hart_id * 8)) as _)
    }
}

/// Universal Asynchronous Receiver/Transmitter (UART) constants
pub mod uart
{
    use crate::soc::Register;

    pub const IRQ: u32 = 10;

    pub const BASE: usize = 0x1000_0000;

    pub const RBR: Register<u8> = Register::new(BASE as _); // Receiver Buffer Register (Read only)
    pub const THR: Register<u8> = Register::new(BASE as _); // Transmit Holding Register (Write only)
    pub const IER: Register<u8> = Register::new((BASE + 1) as _); // Interrupt Enable Register
    pub const FCR: Register<u8> = Register::new((BASE + 2) as _); // FIFO Control Register (Write Only)

    pub mod lsr
    {
        use crate::soc::Register;

        pub const ADDR: Register<u8> = Register::new((super::BASE + 5) as _); // Line Status Register

        pub const RX_READY: u8 = 1 << 0; // The Data Ready bit
        pub const TX_IDLE: u8 = 1 << 5; // The Transmit Holding Register Empty bit
    }
}
