# Arm PrimeCell UART (PL011) driver

Driver implementation for the [PL011 UART peripheral](https://developer.arm.com/documentation/ddi0183/latest/).

The main concept of the driver is that the `Uart` implementation expects a `Deref<Target = PL011Registers>` type. This
type can be an actual reference of a `PL011Registers` structure or any other PAC (Peripheral Access Crate) specific
type as long as it implements the required `Deref` trait. This also allows having custom wrappers around
`PL011Registers` for project specific reasons, like virtual memory mapping of peripherals.

## Implemented features

* Enabling/disabling UART peripheral which includes configuring the data bit number, parity settings, stop bit count
  and baudrate
* Checking various status flags
* Non-blocking read/write functions
* Handling UART errors
* Reading UART identification structure
* 98% unit test coverage
* Implementing various traits
  * `embedded_hal_nb::serial::{Write, Read}`
  * `embedded_io::{Write, Read}`
  * `core::fmt::Write`

## Future plans

  * Implementing interrupt enablement
  * Handling modem control and status signals
  * Adding peripheral testing

## License

The project is MIT and Apache-2.0 dual licensed, see `LICENSE-APACHE` and `LICENSE-MIT`.

## Maintainers

rust-spmc is a trustedfirmware.org maintained project. All contributions are ultimately merged by the maintainers
listed below.

* Bálint Dobszay <balint.dobszay@arm.com>
  [balint-dobszay-arm](https://github.com/balint-dobszay-arm)
* Imre Kis <imre.kis@arm.com>
  [imre-kis-arm](https://github.com/imre-kis-arm)
* Sandrine Afsa <sandrine.afsa@arm.com>
  [sandrine-bailleux-arm](https://github.com/sandrine-bailleux-arm)

## Contributing

Please follow the directions of the [Trusted Firmware Processes](https://trusted-firmware-docs.readthedocs.io/en/latest/generic_processes/index.html)

## Reporting Security Issues

Please follow the directions of the [Trusted Firmware Security Center](https://trusted-firmware-docs.readthedocs.io/en/latest/security_center/index.html)

--------------

*Copyright 2024 Arm Limited and/or its affiliates <open-source-office@arm.com>*
