# cp211x_uart

HID-to-UART driver for CP2110/CP2114 chipset.
It is wrapper around `hidapi::HidDevice` intrinsically.

# Dependencies

* `hidapi` crate.

# Usage

``` rust
extern crate hidapi;
extern crate cp211x_uart;

use std::time::Duration;
use cp211x_uart::{HidUart, UartConfig, DataBits, StopBits, Parity, FlowControl};

fn main() {
    let api = hidapi::HidApi::new().unwrap();
    let device = api.open(0x10C4, 0xEA80).unwrap();
    let mut uart = HidUart::new(device).unwrap();

    let config = UartConfig {
        baud_rate: 9600,
        data_bits: DataBits::Bits8,
        stop_bits: StopBits::Short,
        parity: Parity::None,
        flow_control: FlowControl::None,
    };

    uart.set_config(&config).unwrap();
    uart.set_read_timeout(Duration::from_millis(50));
    uart.set_write_timeout(Duration::from_millis(500));
    uart.flush_fifos(true, true);

    uart.write(&[0x01, 0x02, 0x03][..]).unwrap();
    let mut buf: [u8; 256] = [0; 256];
    uart.read(&mut buf).unwrap();
}
```
