// SPDX-FileCopyrightText: Copyright 2023-2024 Arm Limited and/or its affiliates <open-source-office@arm.com>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Arm PrimeCell UART (PL011) driver
//!
//! Driver implementation for the [PL011 UART peripheral](https://developer.arm.com/documentation/ddi0183/latest/).

#![no_std]

use bitflags::bitflags;
use core::ops::Deref;
use embedded_hal_nb::nb;
use embedded_hal_nb::serial;
use volatile_register::{RO, RW, WO};

// Register descriptions

bitflags! {
    // Data Register
    #[repr(transparent)]
    #[derive(Copy, Clone)]
    struct DataRegister : u32 {
        /// Overrun error
        const OE = 1 << 11;
        /// Break error
        const BE = 1 << 10;
        /// Parity error
        const PE = 1 << 9;
        /// Framing error
        const FE = 1 << 8;
        /// Data
        const DATA_MASK = 0xff;
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
    uartdr: RW<u32>,                    // 0x000 Data Register
    uartrsr_ecr: RW<u32>,               // 0x004 Receive Status Register/Error Clear Register
    reserved_08: [u32; 4],              // 0x008 - 0x014
    uartfr: RO<FlagsRegister>,          // 0x018 Flag Register
    reserved_1c: u32,                   // 0x01C
    uartilpr: RW<u32>,                  // 0x020 IrDA Low-Power Counter Register
    uartibrd: RW<u32>,                  // 0x024 Integer Baud Rate Register
    uartfbrd: RW<u32>,                  // 0x028 Fractional Baud Rate Register
    uartlcr_h: RW<LineControlRegister>, // 0x02C Line Control Register
    uartcr: RW<ControlRegister>,        // 0x030 Control Register
    uartifls: RW<u32>,                  // 0x034 Interrupt FIFO Level Select Register
    uartimsc: RW<u32>,                  // 0x038 Interrupt Mask Set/Clear Register
    uartris: RW<u32>,                   // 0x03C Raw Interrupt Status Register
    uartmis: RW<u32>,                   // 0x040 Masked INterrupt Status Register
    uarticr: WO<u32>,                   // 0x044 Interrupt Clear Register
    uartdmacr: RW<u32>,                 // 0x048 DMA control Register
    reserved_4c: [u32; 997],            // 0x04C - 0xFDC
    uartperiphid0: RO<u32>,             // 0xFE0 UARTPeriphID0 Register
    uartperiphid1: RO<u32>,             // 0xFE4 UARTPeriphID1 Register
    uartperiphid2: RO<u32>,             // 0xFE8 UARTPeriphID2 Register
    uartperiphid3: RO<u32>,             // 0xFEC UARTPeriphID3 Register
    uartpcellid0: RO<u32>,              // 0xFF0 UARTPCellID0 Register
    uartpcellid1: RO<u32>,              // 0xFF4 UARTPCellID1 Register
    uartpcellid2: RO<u32>,              // 0xFF8 UARTPCellID2 Register
    uartpcellid3: RO<u32>,              // 0xFFC UARTPCellID3 Register
}

// Config

/// Data bit count
pub enum DataBits {
    Bits5,
    Bits6,
    Bits7,
    Bits8,
}

/// Parity
pub enum Parity {
    None,
    Even,
    Odd,
    One,
    Zero,
}

/// Stop bit count
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

/// PL011 UART error type
#[derive(Debug)]
pub enum Error {
    InvalidParameter,
    Overrun,
    Break,
    Parity,
    Framing,
}

/// PL011 UART implementation
pub struct Uart<R>
where
    R: Deref<Target = PL011Registers>,
{
    regs: R,
}

impl<R> Uart<R>
where
    R: Deref<Target = PL011Registers>,
{
    /// Create new UART instance
    pub fn new(regs: R) -> Self {
        Self { regs }
    }

    /// Configure and enable uart
    pub fn enable(&self, config: LineConfig, baud_rate: u32, sysclk: u32) -> Result<(), Error> {
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

        unsafe {
            self.regs.uartrsr_ecr.write(0);
            self.regs.uartcr.write(ControlRegister::empty());

            self.regs.uartibrd.write(uartibrd);
            self.regs.uartfbrd.write(uartfbrd);
            self.regs.uartlcr_h.write(line_control);

            self.regs
                .uartcr
                .write(ControlRegister::RXE | ControlRegister::TXE | ControlRegister::UARTEN);
        }

        Ok(())
    }

    /// Disable UART
    pub fn disable(&self) {
        unsafe {
            self.regs.uartcr.write(ControlRegister::empty());
        }
    }

    /// Check if receive FIFO is empty
    pub fn is_rx_fifo_empty(&self) -> bool {
        self.regs.uartfr.read().contains(FlagsRegister::RXFE)
    }

    /// Check if receive FIFO is full
    pub fn is_rx_fifo_full(&self) -> bool {
        self.regs.uartfr.read().contains(FlagsRegister::RXFF)
    }

    /// Check if transmit FIFO is empty
    pub fn is_tx_fifo_empty(&self) -> bool {
        self.regs.uartfr.read().contains(FlagsRegister::TXFE)
    }

    /// Check if transmit FIFO is full
    pub fn is_tx_fifo_full(&self) -> bool {
        self.regs.uartfr.read().contains(FlagsRegister::TXFF)
    }

    /// Check if UART is busy
    pub fn is_busy(&self) -> bool {
        self.regs.uartfr.read().contains(FlagsRegister::BUSY)
    }

    /// Read single byte from the UART
    pub fn read_word(&self) -> Result<u8, Error> {
        let dr = self.regs.uartdr.read();

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

    /// Write single byte to the UART
    pub fn write_word(&self, word: u8) {
        unsafe {
            self.regs.uartdr.write(word as u32);
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

        if ibrd == 0 || (ibrd == 0xffff && fbrd != 0) {
            return Err(Error::InvalidParameter);
        }

        Ok((ibrd, fbrd))
    }
}

// embedded-nb implementation

impl<R> serial::ErrorType for Uart<R>
where
    R: Deref<Target = PL011Registers>,
{
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
    R: Deref<Target = PL011Registers>,
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
    R: Deref<Target = PL011Registers>,
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
impl<R> embedded_io::ErrorType for Uart<R>
where
    R: Deref<Target = PL011Registers>,
{
    type Error = Error;
}

impl embedded_io::Error for Error {
    fn kind(&self) -> embedded_io::ErrorKind {
        embedded_io::ErrorKind::Other
    }
}

impl<R> embedded_io::Write for Uart<R>
where
    R: Deref<Target = PL011Registers>,
{
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        for word in buf {
            match serial::Write::write(self, *word) {
                Err(nb::Error::Other(err)) => return Err(err),
                _ => continue,
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
    R: Deref<Target = PL011Registers>,
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
    R: Deref<Target = PL011Registers>,
{
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        embedded_io::Write::write(self, s.as_bytes()).map_err(|_| core::fmt::Error)?;
        Ok(())
    }
}

#[test]
fn test_pl011_size() {
    assert_eq!(core::mem::size_of::<PL011Registers>(), 0x1000);
}
