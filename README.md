# Arm PrimeCell UART (PL011) driver

Driver implementation for the [PL011 UART peripheral](https://developer.arm.com/documentation/ddi0183/latest/).

The driver is designed to function regardless of whether a Memory Management Unit (MMU) is present.
The primary role of the `OwnedMmioPointer` is to manage the lifetime of the peripheral, ensuring
proper resource handling. In a system that includes an MMU, the peripheral's lifetime is dynamic
because it is mapped into memory rather than having a fixed address. In a system without an MMU, the
`OwnedMmioPointer` can be instantiated directly from the physical address of the register block,
providing access to the peripheral without requiring memory mapping.

## Implemented features

* Enabling/disabling UART peripheral which includes configuring the data bit number, parity settings, stop bit count
  and baudrate
* Checking various status flags
* Non-blocking read/write functions
* Handling UART errors
* Reading UART identification structure
* 98% unit test coverage
* Setting FIFO level of RX/TX interrupts
* Reading, masking and clearing interrupts
* Implementing various traits
  * `embedded_hal_nb::serial::{Write, Read}` (optional, behind the `embedded-hal-nb` feature flag)
  * `embedded_io::{Write, Read}` (optional, behind the `embedded-io` feature flag)
  * `core::fmt::Write`

## Feature flags

* `embedded-hal-nb`: Adds implementations of `embedded-hal-nb` traits for the UART driver.
* `embedded-io`: Adds implementations of `embedded-io` traits for the UART driver.

## Future plans

* Handling modem control and status signals
* Adding peripheral testing

## Example

```rust
use arm_pl011_uart::{DataBits, LineConfig, Parity, PL011Registers, StopBits, Uart, UniqueMmioPointer};
use core::{fmt::Write, ptr::NonNull};
# use zerocopy::transmute_mut;
# let mut fake_registers = [0u32; 1024];
# let UART_ADDRESS : *mut PL011Registers = transmute_mut!(&mut fake_registers);

// SAFETY: `UART_ADDRESS` is the base address of a PL011 UART register block. It remains valid for
// the lifetime of the application and nothing else references this address range.
let uart_pointer = unsafe { UniqueMmioPointer::new(NonNull::new(UART_ADDRESS).unwrap()) };

// Create driver instance
let mut uart = Uart::new(uart_pointer);

// Configure and enable UART
let line_config = LineConfig {
    data_bits: DataBits::Bits8,
    parity: Parity::None,
    stop_bits: StopBits::One,
};
uart.enable(line_config, 115_200, 16_000_000);

// Send and receive data
uart.write_word(0x5a);
uart.write_str("Hello Uart!");
println!("{:?}", uart.read_word());
```

## License

The project is MIT and Apache-2.0 dual licensed, see `LICENSE-APACHE` and `LICENSE-MIT`.

## Maintainers

arm-pl011-uart is a trustedfirmware.org maintained project. All contributions are ultimately merged by the maintainers
listed below.

* Bálint Dobszay <balint.dobszay@arm.com>
  [balint-dobszay-arm](https://github.com/balint-dobszay-arm)
* Imre Kis <imre.kis@arm.com>
  [imre-kis-arm](https://github.com/imre-kis-arm)
* Sandrine Afsa <sandrine.afsa@arm.com>
  [sandrine-bailleux-arm](https://github.com/sandrine-bailleux-arm)

## Contributing

Please follow the directions of the [Trusted Firmware Processes](https://trusted-firmware-docs.readthedocs.io/en/latest/generic_processes/index.html)

Contributions are handled through [review.trustedfirmware.org](https://review.trustedfirmware.org/q/project:rust-spmc/arm-pl011-uart).

## Reporting Security Issues

Please follow the directions of the [Trusted Firmware Security Center](https://trusted-firmware-docs.readthedocs.io/en/latest/security_center/index.html)

--------------

*Copyright 2024 Arm Limited and/or its affiliates <open-source-office@arm.com>*
