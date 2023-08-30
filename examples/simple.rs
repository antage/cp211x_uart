extern crate cp211x_uart;
extern crate hidapi;

use cp211x_uart::{DataBits, FlowControl, HidUart, Parity, StopBits, UartConfig};
use std::time::Duration;

fn run() -> Result<(), cp211x_uart::Error> {
    let manager = hidapi::HidApi::new()?;
    for device_info in manager.device_list().filter(|device_info| {
        device_info.vendor_id() == 0x10C4 && device_info.product_id() == 0xEA80
    }) {
        let handle = device_info.open_device(&manager)?;
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
