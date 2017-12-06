# cp211x_uart

[![Documentation](https://docs.rs/cp211x_uart/badge.svg)](https://docs.rs/cp211x_uart) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT) [![Build Status](https://travis-ci.org/antage/cp211x_uart.svg?branch=master)](https://travis-ci.org/antage/cp211x_uart) [![Build status](https://ci.appveyor.com/api/projects/status/mdiv626vqq496tuu?svg=true)](https://ci.appveyor.com/project/antage/cp211x-uart)

HID-to-UART driver for CP2110/CP2114 chipset.
It is wrapper around `hid::Handle` intrinsically.

See [documentation](https://docs.rs/cp211x_uart) for details.

## Building

Linux:

```
$ sudo apt-get install libudev-dev libhidapi-dev
$ cargo build
```

## Usage

``` rust
extern crate hid;
extern crate cp211x_uart;

use std::time::Duration;
use cp211x_uart::{HidUart, UartConfig, DataBits, StopBits, Parity, FlowControl};

fn run() -> Result<(), cp211x_uart::Error> {
    let manager = hid::init()?;
    for device in manager.find(Some(0x10C4), Some(0xEA80)) {
        let handle = device.open()?;
        let mut uart = HidUart::new(handle)?;

        let config = UartConfig {
            baud_rate: 9600,
            data_bits: DataBits::Bits8,
            stop_bits: StopBits::Short,
            parity: Parity::None,
            flow_control: FlowControl::None,
        };

        uart.set_config(&config)?;
        uart.set_read_timeout(Duration::from_millis(50));
        uart.set_write_timeout(Duration::from_millis(500));
        uart.flush_fifos(true, true)?;

        uart.write(&[0x01, 0x02, 0x03][..])?;
        let mut buf: [u8; 256] = [0; 256];
        uart.read(&mut buf)?;
    }
}

fn main() {
    match run() {
        Ok() => {}
        Err(err) => {
            eprintln!("ERROR: {}", err);
        }
    }
}
```

## License

This library licensed under the following:

* MIT License ([LICENSE](LICENSE) or https://opensource.org/licenses/MIT)
