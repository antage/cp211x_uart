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
    Ok(())
}

fn main() {
    match run() {
        Err(err) => {
            eprintln!("ERROR: {}", err);
        }
        _ => {}
    }
}