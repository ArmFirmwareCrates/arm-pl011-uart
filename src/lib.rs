// SPDX-FileCopyrightText: Copyright 2023-2024 Arm Limited and/or its affiliates <open-source-office@arm.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_std]
#![doc = include_str!("../README.md")]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]

use bitflags::bitflags;
use embedded_hal_nb::nb;
use embedded_hal_nb::serial;
use thiserror::Error;

// Register descriptions

bitflags! {
    /// Data Register
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct DataRegister: u32 {
        /// Overrun error
        const OE = 1 << 11;
        /// Break error
        const BE = 1 << 10;
        /// Parity error
        const PE = 1 << 9;
        /// Framing error
        const FE = 1 << 8;
    }

    /// Receive Status Register/Error Clear Register, UARTRSR/UARTECR
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct ReceiveStatusRegister : u32 {
        /// Overrun error
        const OE = 1 << 3;
        /// Break error
        const BE = 1 << 2;
        /// Parity error
        const PE = 1 << 1;
        /// Framing error
        const FE = 1 << 0;
    }

    /// Flag Register, UARTFR
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct FlagsRegister: u32 {
        /// Ring indicator
        const RI = 1 << 8;
        /// Transmit FIFO is empty
        const TXFE = 1 << 7;
        /// Receive FIFO is full
        const RXFF = 1 << 6;
        /// Transmit FIFO is full
        const TXFF = 1 << 5;
        /// Receive FIFO is empty
        const RXFE = 1 << 4;
        /// UART busy
        const BUSY = 1 << 3;
        /// Data carrier detect
        const DCD = 1 << 2;
        /// Data set ready
        const DSR = 1 << 1;
        /// Clear to send
        const CTS = 1 << 0;
    }

    /// Line Control Register, UARTLCR_H
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct LineControlRegister: u32 {
        /// Stick parity select.
        const SPS = 1 << 7;
        /// Word length
        const WLEN_5BITS = 0b00 << 5;
        const WLEN_6BITS = 0b01 << 5;
        const WLEN_7BITS = 0b10 << 5;
        const WLEN_8BITS = 0b11 << 5;
        /// Enable FIFOs
        const FEN = 1 << 4;
        /// Two stop bits select
        const STP2 = 1 << 3;
        /// Even parity select
        const EPS = 1 << 2;
        /// Parity enable
        const PEN = 1 << 1;
        /// Send break
        const BRK = 1 << 0;
    }

     /// Control Register, UARTCR
     #[repr(transparent)]
     #[derive(Copy, Clone)]
     struct ControlRegister: u32 {
        /// CTS hardware flow control enable
        const CTSEn = 1 << 15;
        /// RTS hardware flow control enable
        const RTSEn = 1 << 14;
        /// This bit is the complement of the UART Out2 (nUARTOut2) modem status output
        const Out2 = 1 << 13;
        /// This bit is the complement of the UART Out1 (nUARTOut1) modem status output
        const Out1 = 1 << 12;
        /// Request to send
        const RTS = 1 << 11;
        /// Data transmit ready
        const DTR = 1 << 10;
        /// Receive enable
        const RXE = 1 << 9;
        /// Transmit enable
        const TXE = 1 << 8;
        /// Loopback enable
        const LBE = 1 << 7;
        /// SIR low-power IrDA mode
        const SIRLP = 1 << 2;
        /// SIR enable
        const SIREN = 1 << 1;
        /// UART enable
        const UARTEN = 1 << 0;
     }
}

/// PL011 register map
#[repr(C, align(4))]
pub struct PL011Registers {
    /// 0x000: Data Register
    uartdr: u32,
    /// 0x004: Receive Status Register/Error Clear Register
    uartrsr_ecr: u32,
    /// 0x008 - 0x014
    reserved_08: [u32; 4],
    /// 0x018: Flag Register
    uartfr: FlagsRegister,
    /// 0x01C
    reserved_1c: u32,
    /// 0x020: IrDA Low-Power Counter Register
    uartilpr: u32,
    /// 0x024: Integer Baud Rate Register
    uartibrd: u32,
    /// 0x028: Fractional Baud Rate Register
    uartfbrd: u32,
    /// 0x02C: Line Control Register
    uartlcr_h: LineControlRegister,
    /// 0x030: Control Register
    uartcr: ControlRegister,
    /// 0x034: Interrupt FIFO Level Select Register
    uartifls: u32,
    /// 0x038: Interrupt Mask Set/Clear Register
    uartimsc: u32,
    /// 0x03C: Raw Interrupt Status Register
    uartris: u32,
    /// 0x040: Masked INterrupt Status Register
    uartmis: u32,
    /// 0x044: Interrupt Clear Register
    uarticr: u32,
    /// 0x048: DMA control Register
    uartdmacr: u32,
    /// 0x04C - 0xFDC
    reserved_4c: [u32; 997],
    /// 0xFE0: UARTPeriphID0 Register
    uartperiphid0: u32,
    /// 0xFE4: UARTPeriphID1 Register
    uartperiphid1: u32,
    /// 0xFE8: UARTPeriphID2 Register
    uartperiphid2: u32,
    /// 0xFEC: UARTPeriphID3 Register
    uartperiphid3: u32,
    /// 0xFF0: UARTPCellID0 Register
    uartpcellid0: u32,
    /// 0xFF4: UARTPCellID1 Register
    uartpcellid1: u32,
    /// 0xFF8: UARTPCellID2 Register
    uartpcellid2: u32,
    /// 0xFFC: UARTPCellID3 Register
    uartpcellid3: u32,
}

// Config

/// Data bit count
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataBits {
    Bits5,
    Bits6,
    Bits7,
    Bits8,
}

/// Parity
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Parity {
    None,
    Even,
    Odd,
    One,
    Zero,
}

/// Stop bit count
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopBits {
    One,
    Two,
}

/// UART line config structure
pub struct LineConfig {
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
}

/// UART peripheral identification structure
pub struct Identification {
    part_number: u16,
    designer: u8,
    revision_number: u8,
    configuration: u8,
}

impl Identification {
    const PART_NUMBER: u16 = 0x11;
    const DESIGNER_ARM: u8 = b'A';
    const REVISION_MAX: u8 = 0x03;
    const CONFIGURATION: u8 = 0x00;

    pub fn is_valid(&self) -> bool {
        self.part_number == Self::PART_NUMBER
            && self.designer == Self::DESIGNER_ARM
            && self.revision_number <= Self::REVISION_MAX
            && self.configuration == Self::CONFIGURATION
    }
}

/// PL011 UART error type
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum Error {
    #[error("Invalid parameter")]
    InvalidParameter,
    #[error("Overrun")]
    Overrun,
    #[error("Break")]
    Break,
    #[error("Parity")]
    Parity,
    #[error("Framing")]
    Framing,
}

/// Trait to provide access to the registers of a PL011 UART.
pub trait RegisterAccessor {
    fn ptr(&self) -> *const PL011Registers;
    fn ptr_mut(&mut self) -> *mut PL011Registers;
}

impl RegisterAccessor for *mut PL011Registers {
    fn ptr(&self) -> *const PL011Registers {
        *self
    }

    fn ptr_mut(&mut self) -> *mut PL011Registers {
        *self
    }
}

/// PL011 UART implementation
pub struct Uart<R> {
    regs: R,
}

impl<R> Uart<R>
where
    R: RegisterAccessor,
{
    /// Create new UART instance
    ///
    /// # Safety
    ///
    /// The instance of `R` passed must always return pointers to the registers of a PL011 UART
    /// which are properly aligned, mapped and valid to read and write from any thread with volatile
    /// operations. They must not have any other aliases which are used to access them while the methods of this
    /// `Uart` are called.
    ///
    /// In the case that `R` is a `*mut PL011Registers` this means that it must be valid and unique,
    /// and remain so for the lifetime of the `Uart`.
    pub unsafe fn new(regs: R) -> Self {
        Self { regs }
    }

    /// Configure and enable UART
    pub fn enable(&mut self, config: LineConfig, baud_rate: u32, sysclk: u32) -> Result<(), Error> {
        // Baud rate
        let (uartibrd, uartfbrd) = Self::calculate_baud_rate_divisor(baud_rate, sysclk)?;

        // Line control register
        let line_control = match config.data_bits {
            DataBits::Bits5 => LineControlRegister::WLEN_5BITS,
            DataBits::Bits6 => LineControlRegister::WLEN_6BITS,
            DataBits::Bits7 => LineControlRegister::WLEN_7BITS,
            DataBits::Bits8 => LineControlRegister::WLEN_8BITS,
        } | match config.parity {
            Parity::None => LineControlRegister::empty(),
            Parity::Even => LineControlRegister::PEN | LineControlRegister::EPS,
            Parity::Odd => LineControlRegister::PEN,
            Parity::One => LineControlRegister::PEN | LineControlRegister::SPS,
            Parity::Zero => {
                LineControlRegister::PEN | LineControlRegister::EPS | LineControlRegister::SPS
            }
        } | match config.stop_bits {
            StopBits::One => LineControlRegister::empty(),
            StopBits::Two => LineControlRegister::STP2,
        } | LineControlRegister::FEN;

        // SAFETY: The caller of Uart::new promised that self.regs will return a valid PL011
        // register block.
        unsafe {
            (&raw mut (*self.regs.ptr_mut()).uartrsr_ecr).write_volatile(0);
            (&raw mut (*self.regs.ptr_mut()).uartcr).write_volatile(ControlRegister::empty());

            (&raw mut (*self.regs.ptr_mut()).uartibrd).write_volatile(uartibrd);
            (&raw mut (*self.regs.ptr_mut()).uartfbrd).write_volatile(uartfbrd);
            (&raw mut (*self.regs.ptr_mut()).uartlcr_h).write_volatile(line_control);

            (&raw mut (*self.regs.ptr_mut()).uartcr).write_volatile(
                ControlRegister::RXE | ControlRegister::TXE | ControlRegister::UARTEN,
            );
        }

        Ok(())
    }

    /// Disable UART
    pub fn disable(&mut self) {
        // SAFETY: The caller of Uart::new promised that self.regs will return a valid PL011
        // register block.
        unsafe {
            (&raw mut (*self.regs.ptr_mut()).uartcr).write_volatile(ControlRegister::empty());
        }
    }

    /// Check if receive FIFO is empty
    pub fn is_rx_fifo_empty(&self) -> bool {
        self.flags().contains(FlagsRegister::RXFE)
    }

    /// Check if receive FIFO is full
    pub fn is_rx_fifo_full(&self) -> bool {
        self.flags().contains(FlagsRegister::RXFF)
    }

    /// Check if transmit FIFO is empty
    pub fn is_tx_fifo_empty(&self) -> bool {
        self.flags().contains(FlagsRegister::TXFE)
    }

    /// Check if transmit FIFO is full
    pub fn is_tx_fifo_full(&self) -> bool {
        self.flags().contains(FlagsRegister::TXFF)
    }

    /// Check if UART is busy
    pub fn is_busy(&self) -> bool {
        self.flags().contains(FlagsRegister::BUSY)
    }

    /// Reads and returns the flag register.
    fn flags(&self) -> FlagsRegister {
        // SAFETY: The caller of Uart::new promised that self.regs will return a valid PL011
        // register block.
        unsafe { (&raw const (*self.regs.ptr()).uartfr).read_volatile() }
    }

    /// Non-blocking read of a single byte from the UART
    pub fn read_word(&self) -> Result<u8, Error> {
        // SAFETY: The caller of Uart::new promised that self.regs will return a valid PL011
        // register block.
        let dr = unsafe { (&raw const (*self.regs.ptr()).uartdr).read_volatile() };

        let flags = DataRegister::from_bits_truncate(dr);

        if flags.contains(DataRegister::OE) {
            return Err(Error::Overrun);
        } else if flags.contains(DataRegister::BE) {
            return Err(Error::Break);
        } else if flags.contains(DataRegister::PE) {
            return Err(Error::Parity);
        } else if flags.contains(DataRegister::FE) {
            return Err(Error::Framing);
        }

        Ok(dr as u8)
    }

    /// Non-blocking write of a single byte to the UART
    pub fn write_word(&mut self, word: u8) {
        // SAFETY: The caller of Uart::new promised that self.regs will return a valid PL011
        // register block.
        unsafe {
            (&raw mut (*self.regs.ptr_mut()).uartdr).write_volatile(word as u32);
        }
    }

    /// Read UART peripheral identification structure
    pub fn read_identification(&self) -> Identification {
        // SAFETY: The caller of Uart::new promised that self.regs will return a valid PL011
        // register block.
        let id: [u32; 4] = unsafe {
            [
                (&raw const (*self.regs.ptr()).uartperiphid0).read_volatile(),
                (&raw const (*self.regs.ptr()).uartperiphid1).read_volatile(),
                (&raw const (*self.regs.ptr()).uartperiphid2).read_volatile(),
                (&raw const (*self.regs.ptr()).uartperiphid3).read_volatile(),
            ]
        };

        Identification {
            part_number: (id[0] & 0xff) as u16 | ((id[1] & 0x0f) << 8) as u16,
            designer: ((id[1] & 0xf0) >> 4) as u8 | ((id[2] & 0x0f) << 4) as u8,
            revision_number: ((id[2] & 0xf0) >> 4) as u8,
            configuration: (id[3] & 0xff) as u8,
        }
    }

    fn calculate_baud_rate_divisor(baud_rate: u32, sysclk: u32) -> Result<(u32, u32), Error> {
        // baud_div = sysclk / (baud_rate * 16)
        // baud_div_bits = (baud_div * 2^7 + 1) / 2
        // After simplifying:
        // baud_div_bits = ((sysclk * 8 / baud_rate) + 1) / 2
        let baud_div = sysclk
            .checked_mul(8)
            .and_then(|clk| clk.checked_div(baud_rate))
            .ok_or(Error::InvalidParameter)?;
        let baud_div_bits = baud_div
            .checked_add(1)
            .map(|div| div >> 1)
            .ok_or(Error::InvalidParameter)?;

        let ibrd = baud_div_bits >> 6;
        let fbrd = baud_div_bits & 0x3F;

        if ibrd == 0 || (ibrd == 0xffff && fbrd != 0) || ibrd > 0xffff {
            return Err(Error::InvalidParameter);
        }

        Ok((ibrd, fbrd))
    }
}

// SAFETY: The caller of `Uart::new` promises that the UART's registers can be accessed from any
// thread.
unsafe impl<R: RegisterAccessor> Send for Uart<R> {}

// SAFETY: An `&Uart<R>` only allows operations which read registers, which can safely be done from
// multiple threads simultaneously.
unsafe impl<R: RegisterAccessor> Sync for Uart<R> {}

// embedded-nb implementation

impl<R> serial::ErrorType for Uart<R> {
    type Error = Error;
}

impl serial::Error for Error {
    fn kind(&self) -> serial::ErrorKind {
        match self {
            Error::InvalidParameter => serial::ErrorKind::Other,
            Error::Overrun => serial::ErrorKind::Overrun,
            Error::Break => serial::ErrorKind::Other,
            Error::Parity => serial::ErrorKind::Parity,
            Error::Framing => serial::ErrorKind::FrameFormat,
        }
    }
}

impl<R> serial::Write for Uart<R>
where
    R: RegisterAccessor,
{
    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        if self.is_tx_fifo_full() {
            return Err(nb::Error::WouldBlock);
        }

        self.write_word(word);

        Ok(())
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        if self.is_busy() {
            Err(nb::Error::WouldBlock)
        } else {
            Ok(())
        }
    }
}

impl<R> serial::Read for Uart<R>
where
    R: RegisterAccessor,
{
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        if self.is_rx_fifo_empty() {
            return Err(nb::Error::WouldBlock);
        }

        match self.read_word() {
            Ok(word) => Ok(word),
            Err(err) => Err(nb::Error::Other(err)),
        }
    }
}

// embedded-io implementation
impl<R> embedded_io::ErrorType for Uart<R> {
    type Error = Error;
}

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        match self {
            Self::Break | Self::Overrun => embedded_io::ErrorKind::Other,
            Self::Framing | Self::Parity => embedded_io::ErrorKind::InvalidData,
            Self::InvalidParameter => embedded_io::ErrorKind::InvalidInput,
        }
    }
}

impl<R> embedded_io::Write for Uart<R>
where
    R: RegisterAccessor,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for word in buf {
            loop {
                match serial::Write::write(self, *word) {
                    Err(nb::Error::Other(err)) => return Err(err),
                    Err(nb::Error::WouldBlock) => continue,
                    Ok(()) => break,
                }
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        loop {
            match serial::Write::flush(self) {
                Ok(_) => return Ok(()),
                Err(nb::Error::Other(err)) => return Err(err),
                _ => continue,
            }
        }
    }
}

impl<R> embedded_io::Read for Uart<R>
where
    R: RegisterAccessor,
{
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let mut index = 0;

        while index != buf.len() {
            match serial::Read::read(self) {
                Ok(byte) => {
                    buf[index] = byte;
                    index += 1;
                }
                Err(nb::Error::Other(err)) => return Err(err),
                Err(nb::Error::WouldBlock) => continue,
            }
        }

        Ok(buf.len())
    }
}

// core::fmt::Write implementation

impl<R> core::fmt::Write for Uart<R>
where
    R: RegisterAccessor,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        embedded_io::Write::write(self, s.as_bytes()).map_err(|_| core::fmt::Error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakePL011Registers {
        regs: [u32; 1024],
    }

    impl FakePL011Registers {
        fn new() -> Self {
            Self { regs: [0u32; 1024] }
        }

        fn clear(&mut self) {
            self.regs.fill(0);
        }

        fn reg_write(&mut self, offset: usize, value: u32) {
            self.regs[offset / 4] = value;
        }

        fn reg_read(&self, offset: usize) -> u32 {
            self.regs[offset / 4]
        }

        fn get(&mut self) -> PL011RegsRef {
            PL011RegsRef {
                regs: &mut self.regs,
            }
        }

        fn uart_for_test(&mut self) -> Uart<PL011RegsRef> {
            // SAFETY: PL011RegsRef always returns valid pointers because they come from a reference
            // tied to its lifetime.
            unsafe { Uart::new(self.get()) }
        }
    }

    struct PL011RegsRef<'a> {
        regs: &'a mut [u32; 1024],
    }

    impl RegisterAccessor for PL011RegsRef<'_> {
        fn ptr(&self) -> *const PL011Registers {
            let regs_ptr = self.regs.as_ptr().cast::<PL011Registers>();
            assert!(regs_ptr.is_aligned());

            // regs_ptr points to a FakePL011Registers struct's regs field, that has the same size
            // and alignment as PL011Registers
            regs_ptr
        }

        fn ptr_mut(&mut self) -> *mut PL011Registers {
            let regs_ptr = self.regs.as_mut_ptr().cast::<PL011Registers>();
            assert!(regs_ptr.is_aligned());

            // regs_ptr points to a FakePL011Registers struct's regs field, that has the same size
            // and alignment as PL011Registers
            regs_ptr
        }
    }

    #[test]
    fn regs_size() {
        assert_eq!(core::mem::size_of::<PL011Registers>(), 0x1000);
    }

    #[test]
    fn enable_230400_8n1() {
        let mut regs = FakePL011Registers::new();
        let mut uart = regs.uart_for_test();
        let config = LineConfig {
            data_bits: DataBits::Bits8,
            parity: Parity::None,
            stop_bits: StopBits::One,
        };

        // Example 3-1 from PL011 TRM
        assert_eq!(Ok(()), uart.enable(config, 230400, 4_000_000));

        assert_eq!(0x00, regs.reg_read(0x004)); // UARTSR_ECR
        assert_eq!(1, regs.reg_read(0x024)); // UARTIBDR
        assert_eq!(5, regs.reg_read(0x028)); // UARTFBDR
        assert_eq!(0b01110000, regs.reg_read(0x02c)); // UARTLCR_H
        assert_eq!(0x0301, regs.reg_read(0x030)); // UARTCR
    }

    #[test]
    fn enable_example_baudrates() {
        // Table 3-9
        let mut regs = FakePL011Registers::new();

        {
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 230400, 4_000_000));
            assert_eq!(0x1, regs.reg_read(0x024)); // UARTIBDR
            assert_eq!(0x5, regs.reg_read(0x028)); // UARTFBDR
        }

        regs.clear();

        {
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 115200, 4_000_000));
            assert_eq!(0x2, regs.reg_read(0x024)); // UARTIBDR
            assert_eq!(0xb, regs.reg_read(0x028)); // UARTFBDR
        }

        regs.clear();

        {
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 76800, 4_000_000));
            assert_eq!(0x3, regs.reg_read(0x024)); // UARTIBDR
            assert_eq!(0x10, regs.reg_read(0x028)); // UARTFBDR
        }

        regs.clear();

        {
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 38400, 4_000_000));
            assert_eq!(0x6, regs.reg_read(0x024)); // UARTIBDR
            assert_eq!(0x21, regs.reg_read(0x028)); // UARTFBDR
        }

        regs.clear();

        {
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 14400, 4_000_000));
            assert_eq!(0x11, regs.reg_read(0x024)); // UARTIBDR
            assert_eq!(0x17, regs.reg_read(0x028)); // UARTFBDR
        }

        regs.clear();

        {
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 2400, 4_000_000));
            assert_eq!(0x68, regs.reg_read(0x024)); // UARTIBDR
            assert_eq!(0xb, regs.reg_read(0x028)); // UARTFBDR
        }

        regs.clear();

        {
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 110, 4_000_000));
            assert_eq!(0x8e0, regs.reg_read(0x024)); // UARTIBDR
            assert_eq!(0x2f, regs.reg_read(0x028)); // UARTFBDR
        }
    }

    #[test]
    fn enable_invalid_baudrates() {
        let mut regs = FakePL011Registers::new();
        let mut uart = regs.uart_for_test();

        {
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };

            assert_eq!(
                Err(Error::InvalidParameter),
                uart.enable(config, 0, 4_000_000)
            );
        }

        {
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };
            assert_eq!(
                Err(Error::InvalidParameter),
                uart.enable(config, 1, 1048561)
            );
        }

        {
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };
            assert_eq!(
                Err(Error::InvalidParameter),
                uart.enable(config, 1, 100_000_000)
            );
        }

        {
            let config = LineConfig {
                data_bits: DataBits::Bits8,
                parity: Parity::None,
                stop_bits: StopBits::One,
            };
            assert_eq!(Err(Error::InvalidParameter), uart.enable(config, 1, 1));
        }
    }

    #[test]
    fn enable_lineconfigs() {
        let mut regs = FakePL011Registers::new();
        {
            // 8 bits, even parity, 2 stop bits
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits7,
                parity: Parity::Even,
                stop_bits: StopBits::Two,
            };

            assert_eq!(Ok(()), uart.enable(config, 230400, 4_000_000));
            assert_eq!(0b01011110, regs.reg_read(0x02c)); // UARTLCR_H
        }

        regs.clear();

        {
            // 6 bits, odd parity, 1 stop bit
            let mut regs = FakePL011Registers::new();
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits6,
                parity: Parity::Odd,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 230400, 4_000_000));
            assert_eq!(0b00110010, regs.reg_read(0x02c)); // UARTLCR_H
        }

        regs.clear();

        {
            // 5 bits, one parity, 1 stop bit
            let mut regs = FakePL011Registers::new();
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits5,
                parity: Parity::One,
                stop_bits: StopBits::One,
            };

            assert_eq!(Ok(()), uart.enable(config, 230400, 4_000_000));
            assert_eq!(0b10010010, regs.reg_read(0x02c)); // UARTLCR_H
        }

        {
            // 5 bits, zero paraty, 2 stop bit
            let mut regs = FakePL011Registers::new();
            let mut uart = regs.uart_for_test();
            let config = LineConfig {
                data_bits: DataBits::Bits5,
                parity: Parity::Zero,
                stop_bits: StopBits::Two,
            };

            assert_eq!(Ok(()), uart.enable(config, 230400, 4_000_000));
            assert_eq!(0b10011110, regs.reg_read(0x02c)); // UARTLCR_H
        }
    }

    #[test]
    fn disable() {
        let mut regs = FakePL011Registers::new();
        let mut uart = regs.uart_for_test();
        let config = LineConfig {
            data_bits: DataBits::Bits8,
            parity: Parity::None,
            stop_bits: StopBits::One,
        };

        assert_eq!(Ok(()), uart.enable(config, 230400, 4_000_000));
        uart.disable();
        assert_eq!(0, regs.reg_read(0x030)); // UARTCR
    }

    #[test]
    fn rx_fifo_empty() {
        let mut regs = FakePL011Registers::new();
        {
            let uart = regs.uart_for_test();
            assert!(!uart.is_rx_fifo_empty());
        }

        {
            regs.reg_write(0x018, 1 << 4);
            let uart = regs.uart_for_test();
            assert!(uart.is_rx_fifo_empty());
        }
    }

    #[test]
    fn rx_fifo_full() {
        let mut regs = FakePL011Registers::new();
        {
            let uart = regs.uart_for_test();
            assert!(!uart.is_rx_fifo_full());
        }

        {
            regs.reg_write(0x018, 1 << 6);
            let uart = regs.uart_for_test();
            assert!(uart.is_rx_fifo_full());
        }
    }

    #[test]
    fn tx_fifo_empty() {
        let mut regs = FakePL011Registers::new();
        {
            let uart = regs.uart_for_test();
            assert!(!uart.is_tx_fifo_empty());
        }

        {
            regs.reg_write(0x018, 1 << 7);
            let uart = regs.uart_for_test();
            assert!(uart.is_tx_fifo_empty());
        }
    }

    #[test]
    fn tx_fifo_full() {
        let mut regs = FakePL011Registers::new();
        {
            let uart = regs.uart_for_test();
            assert!(!uart.is_tx_fifo_full());
        }

        {
            regs.reg_write(0x018, 1 << 5);
            let uart = regs.uart_for_test();
            assert!(uart.is_tx_fifo_full());
        }
    }

    #[test]
    fn busy() {
        let mut regs = FakePL011Registers::new();
        {
            let uart = regs.uart_for_test();
            assert!(!uart.is_busy());
        }

        {
            regs.reg_write(0x018, 1 << 3);
            let uart = regs.uart_for_test();
            assert!(uart.is_busy());
        }
    }

    #[test]
    fn read_word() {
        let mut regs = FakePL011Registers::new();

        {
            regs.reg_write(0x000, 1 << 11);

            let uart = regs.uart_for_test();
            assert_eq!(Err(Error::Overrun), uart.read_word());
        }

        {
            regs.reg_write(0x000, 1 << 10);

            let uart = regs.uart_for_test();
            assert_eq!(Err(Error::Break), uart.read_word());
        }

        {
            regs.reg_write(0x000, 1 << 9);

            let uart = regs.uart_for_test();
            assert_eq!(Err(Error::Parity), uart.read_word());
        }

        {
            regs.reg_write(0x000, 1 << 8);

            let uart = regs.uart_for_test();
            assert_eq!(Err(Error::Framing), uart.read_word());
        }

        {
            regs.reg_write(0x000, 0x41);

            let uart = regs.uart_for_test();
            assert_eq!(Ok(0x41), uart.read_word());
        }
    }

    #[test]
    fn write_word() {
        let mut regs = FakePL011Registers::new();

        let mut uart = regs.uart_for_test();
        uart.write_word(0x41);

        assert_eq!(0x41, regs.reg_read(0x000));
    }

    #[test]
    fn read_identification() {
        let mut regs = FakePL011Registers::new();

        regs.reg_write(0xfe0, 0x11);
        regs.reg_write(0xfe4, 0x10);
        regs.reg_write(0xfe8, 0x34);
        regs.reg_write(0xfec, 0x00);

        let uart = regs.uart_for_test();
        let identification = uart.read_identification();
        assert_eq!(0x0011, identification.part_number);
        assert_eq!(0x41, identification.designer);
        assert_eq!(0x03, identification.revision_number);
        assert_eq!(0x00, identification.configuration);
        assert!(identification.is_valid());
    }

    #[test]
    fn error_kind() {
        assert_eq!(
            serial::ErrorKind::Other,
            serial::Error::kind(&Error::InvalidParameter)
        );

        assert_eq!(
            serial::ErrorKind::Overrun,
            serial::Error::kind(&Error::Overrun)
        );

        assert_eq!(serial::ErrorKind::Other, serial::Error::kind(&Error::Break));

        assert_eq!(
            serial::ErrorKind::Parity,
            serial::Error::kind(&Error::Parity)
        );

        assert_eq!(
            serial::ErrorKind::FrameFormat,
            serial::Error::kind(&Error::Framing)
        );

        assert_eq!(
            embedded_io::ErrorKind::InvalidData,
            embedded_io::Error::kind(&Error::Framing)
        );
    }

    #[test]
    fn serial_write() {
        let mut regs = FakePL011Registers::new();

        {
            let mut uart = regs.uart_for_test();
            assert_eq!(Ok(()), serial::Write::write(&mut uart, 0x41));
            assert_eq!(0x41, regs.reg_read(0x000));
        }

        regs.clear();

        {
            regs.reg_write(0x018, 1 << 5);
            let mut uart = regs.uart_for_test();
            assert_eq!(
                Err(nb::Error::WouldBlock),
                serial::Write::write(&mut uart, 0x41)
            );
        }

        regs.clear();

        {
            let mut uart = regs.uart_for_test();
            assert_eq!(Ok(()), serial::Write::flush(&mut uart));
        }
        regs.clear();

        {
            regs.reg_write(0x018, 1 << 3);
            let mut uart = regs.uart_for_test();
            assert_eq!(Err(nb::Error::WouldBlock), serial::Write::flush(&mut uart));
        }
    }

    #[test]
    fn serial_read() {
        let mut regs = FakePL011Registers::new();

        {
            regs.reg_write(0x000, 0x41);

            let mut uart = regs.uart_for_test();
            assert_eq!(Ok(0x41), serial::Read::read(&mut uart));
        }

        regs.clear();

        {
            regs.reg_write(0x000, 0x41);

            let mut uart = regs.uart_for_test();
            assert_eq!(Ok(0x41), serial::Read::read(&mut uart));
        }

        regs.clear();

        {
            regs.reg_write(0x000, 1 << 11);

            let mut uart = regs.uart_for_test();
            assert_eq!(
                Err(nb::Error::Other(Error::Overrun)),
                serial::Read::read(&mut uart)
            );
        }

        regs.clear();

        {
            regs.reg_write(0x018, 1 << 4);

            let mut uart = regs.uart_for_test();
            assert_eq!(Err(nb::Error::WouldBlock), serial::Read::read(&mut uart));
        }
    }

    #[test]
    fn embeddeio_write() {
        let mut regs = FakePL011Registers::new();
        let mut uart = regs.uart_for_test();
        assert_eq!(Ok(2), embedded_io::Write::write(&mut uart, &[1, 2]));
        assert_eq!(Ok(()), embedded_io::Write::flush(&mut uart));
    }

    #[test]
    fn embeddeio_read() {
        let mut regs = FakePL011Registers::new();
        let mut uart = regs.uart_for_test();
        let mut data = [0u8; 2];
        assert_eq!(Ok(2), embedded_io::Read::read(&mut uart, &mut data));
    }

    #[test]
    fn core_write() {
        let mut regs = FakePL011Registers::new();
        let mut uart = regs.uart_for_test();
        assert_eq!(Ok(()), core::fmt::Write::write_str(&mut uart, "hello"));
    }
}
