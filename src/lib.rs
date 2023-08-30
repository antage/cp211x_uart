//! HID-to-UART driver for CP2110/CP2114 chipset.
//!
//! See more information [here][1].
//! [1]: https://www.silabs.com/products/interface/usb-bridges/classic-usb-bridges/device.cp2110-f01-gm

extern crate hidapi;
#[macro_use]
extern crate error_chain;

use std::cmp::min;
use std::default::Default;
use std::time::{Duration, Instant};

mod error;
pub use error::Error;
use error::*;

const FEATURE_REPORT_LENGTH: usize = 64;
const INTERRUPT_REPORT_LENGTH: usize = 64;

const GETSET_UART_ENABLE: u8 = 0x41; // Get Set Receive Status
const PURGE_FIFOS: u8 = 0x43; // Purge FIFOs
const GETSET_UART_CONFIG: u8 = 0x50; // Get Set UART Config

const PURGE_TRANSMIT_MASK: u8 = 0x01;
const PURGE_RECEIVE_MASK: u8 = 0x02;

/// The number of data bits in [UART configuration](struct.UartConfig.html).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataBits {
    /// 5 data bits
    Bits5,
    /// 6 data bits
    Bits6,
    /// 7 data bits
    Bits7,
    /// 8 data bits
    Bits8,
}

impl Default for DataBits {
    fn default() -> Self {
        DataBits::Bits8
    }
}

/// The parity in [UART configuration](struct.UartConfig.html).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parity {
    /// No parity.
    None,
    /// Odd parity (sum of data bits is odd).
    Odd,
    /// Even parity (sum of data bits is even).
    Even,
    /// Mark parity (always 1).
    Mark,
    /// Space parity (always 0).
    Space,
}

impl Default for Parity {
    fn default() -> Self {
        Parity::None
    }
}

/// The number of stop bits in [UART configuration](struct.UartConfig.html).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StopBits {
    /// 1 stop bit.
    Short,
    /// 5 data bits: 1.5 stop bits, 6-8 data bits: 2 stop bits.
    Long,
}

impl Default for StopBits {
    fn default() -> Self {
        StopBits::Short
    }
}

/// The type of flow control in [UART configuration](struct.UartConfig.html).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlowControl {
    /// No flow control.
    None,
    /// RTS/CTS hardware flow control.
    RtsCts,
}

impl Default for FlowControl {
    fn default() -> Self {
        FlowControl::None
    }
}

/// UART configuration.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UartConfig {
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub flow_control: FlowControl,
}

impl Default for UartConfig {
    fn default() -> Self {
        UartConfig {
            baud_rate: 9600,
            data_bits: DataBits::default(),
            parity: Parity::default(),
            stop_bits: StopBits::default(),
            flow_control: FlowControl::default(),
        }
    }
}

/// Wrapper around `hidapi::HidDevice` to provide UART control.
pub struct HidUart {
    handle: hidapi::HidDevice,
    read_timeout: Duration,
    write_timeout: Duration,
    rx_buffer: RxBuffer,
}

fn set_uart_enable(handle: &mut hidapi::HidDevice, enable: bool) -> Result<()> {
    let mut buf: [u8; FEATURE_REPORT_LENGTH] = [0; FEATURE_REPORT_LENGTH];

    buf[0] = GETSET_UART_ENABLE;
    if enable {
        buf[1] = 0x01;
    } else {
        buf[1] = 0x00;
    }
    handle.send_feature_report(&buf[..])?;
    Ok(())
}

impl HidUart {
    /// Returns a new instance of `HidUart` from `hidapi::HidDevice`.
    ///
    /// The instance enables UART automatically.
    pub fn new(handle: hidapi::HidDevice) -> Result<HidUart> {
        let mut instance = HidUart {
            handle,
            read_timeout: Duration::from_millis(1000),
            write_timeout: Duration::from_millis(1000),
            rx_buffer: RxBuffer::new(),
        };
        instance.enable()?;
        Ok(instance)
    }

    /// Returns receiving timeout.
    pub fn read_timeout(&self) -> Duration {
        self.read_timeout
    }

    /// Set receiving timeout to `timeout` value.
    pub fn set_read_timeout(&mut self, timeout: Duration) {
        self.read_timeout = timeout;
    }

    /// Returns transmitting timeout.
    pub fn write_timeout(&self) -> Duration {
        self.write_timeout
    }

    /// Set transmitting timeout to `timeout` value.
    pub fn set_write_timeout(&mut self, timeout: Duration) {
        self.write_timeout = timeout;
    }

    /// Enable UART.
    pub fn enable(&mut self) -> Result<()> {
        set_uart_enable(&mut self.handle, true)
    }

    /// Disable UART.
    pub fn disable(&mut self) -> Result<()> {
        set_uart_enable(&mut self.handle, false)
    }

    /// Returns UART state: `true` if UART is enabled, `false` otherwise.
    pub fn is_enabled(&mut self) -> Result<bool> {
        let mut buf: [u8; FEATURE_REPORT_LENGTH] = [0; FEATURE_REPORT_LENGTH];

        buf[0] = GETSET_UART_ENABLE;
        self.handle.get_feature_report(&mut buf[..])?;
        if buf[1] == 0x00 {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    /// Configures UART.
    pub fn set_config(&mut self, config: &UartConfig) -> Result<()> {
        let mut buf: [u8; FEATURE_REPORT_LENGTH] = [0; FEATURE_REPORT_LENGTH];

        buf[0] = GETSET_UART_CONFIG;
        buf[1] = ((config.baud_rate >> 24) & 0xFF) as u8;
        buf[2] = ((config.baud_rate >> 16) & 0xFF) as u8;
        buf[3] = ((config.baud_rate >> 8) & 0xFF) as u8;
        buf[4] = (config.baud_rate & 0xFF) as u8;
        buf[5] = match config.parity {
            Parity::None => 0x00,
            Parity::Odd => 0x01,
            Parity::Even => 0x02,
            Parity::Mark => 0x03,
            Parity::Space => 0x04,
        };
        buf[6] = match config.flow_control {
            FlowControl::None => 0x00,
            FlowControl::RtsCts => 0x01,
        };
        buf[7] = match config.data_bits {
            DataBits::Bits5 => 0x00,
            DataBits::Bits6 => 0x01,
            DataBits::Bits7 => 0x02,
            DataBits::Bits8 => 0x03,
        };
        buf[8] = match config.stop_bits {
            StopBits::Short => 0x00,
            StopBits::Long => 0x01,
        };

        self.handle.send_feature_report(&buf[..])?;
        Ok(())
    }

    /// Returns current UART configuration.
    pub fn get_config(&mut self) -> Result<UartConfig> {
        let mut buf: [u8; FEATURE_REPORT_LENGTH] = [0; FEATURE_REPORT_LENGTH];

        buf[0] = GETSET_UART_CONFIG;
        self.handle.get_feature_report(&mut buf[..])?;

        let baud_rate: u32 = u32::from(buf[1]) << 24
            | u32::from(buf[2]) << 16
            | u32::from(buf[3]) << 8
            | u32::from(buf[4]);
        let parity = match buf[5] {
            0x00 => Ok(Parity::None),
            0x01 => Ok(Parity::Odd),
            0x02 => Ok(Parity::Even),
            0x03 => Ok(Parity::Mark),
            0x04 => Ok(Parity::Space),
            _ => Err("Unknown parity mode"),
        }?;
        let flow_control = match buf[6] {
            0x00 => Ok(FlowControl::None),
            0x01 => Ok(FlowControl::RtsCts),
            _ => Err("Unknown flow control mode"),
        }?;
        let data_bits = match buf[7] {
            0x00 => Ok(DataBits::Bits5),
            0x01 => Ok(DataBits::Bits6),
            0x02 => Ok(DataBits::Bits7),
            0x03 => Ok(DataBits::Bits8),
            _ => Err("Unknown data bits mode"),
        }?;
        let stop_bits = match buf[8] {
            0x00 => Ok(StopBits::Short),
            0x01 => Ok(StopBits::Long),
            _ => Err("Unknown stop bits mode"),
        }?;
        let config = UartConfig {
            baud_rate,
            parity,
            flow_control,
            data_bits,
            stop_bits,
        };

        Ok(config)
    }

    /// Empties receiving and/or transmitting FIFO buffers.
    ///
    /// Flushes receiving FIFO buffer if `rx` is `true`.
    ///
    /// Flushes transmitting FIFO buffer if `tx` is `true`.
    pub fn flush_fifos(&mut self, rx: bool, tx: bool) -> Result<()> {
        let mut buf: [u8; FEATURE_REPORT_LENGTH] = [0; FEATURE_REPORT_LENGTH];

        buf[0] = PURGE_FIFOS;
        if rx {
            buf[1] |= PURGE_RECEIVE_MASK;

            // also dump any buffered data
            self.rx_buffer.clear();
        }
        if tx {
            buf[1] |= PURGE_TRANSMIT_MASK;
        }
        self.handle.send_feature_report(&buf[..])?;

        Ok(())
    }

    /// Transmit `data`.
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut buf: [u8; INTERRUPT_REPORT_LENGTH];

        let start_time = Instant::now();
        for chunk in data.chunks(INTERRUPT_REPORT_LENGTH - 1) {
            buf = [0; INTERRUPT_REPORT_LENGTH];
            buf[0] = chunk.len() as u8;
            buf[1..chunk.len() + 1].copy_from_slice(chunk);
            self.handle.write(&buf[..])?;
            if start_time.elapsed() > self.write_timeout {
                return Err(ErrorKind::WriteTimeout.into());
            }
        }

        Ok(())
    }

    /// Receive `data` and returns a number of read bytes.
    pub fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        // drain any buffered data
        let mut num_bytes_read = self.rx_buffer.read(data);

        // read from usb
        let mut buf: [u8; INTERRUPT_REPORT_LENGTH];
        let start_time = Instant::now();
        loop {
            let data_free = data.len() - num_bytes_read;
            if data_free > 0 {
                buf = [0; INTERRUPT_REPORT_LENGTH];
                let total_read = self.handle.read_timeout(&mut buf, 1)?;
                if total_read != 0 {
                    let report_len: usize = buf[0] as usize;
                    let copy_len = min(report_len, data_free);
                    data[num_bytes_read..(num_bytes_read + copy_len)]
                        .copy_from_slice(&buf[1..(copy_len + 1)]);
                    num_bytes_read += copy_len;

                    // buffer the left overs
                    if copy_len < report_len {
                        let left = report_len - copy_len;
                        let start = 1 + copy_len;
                        let end = start + left;

                        self.rx_buffer.write(&buf[start..end]);

                        return Ok(num_bytes_read);
                    }
                }
            } else {
                break;
            }
            if start_time.elapsed() > self.read_timeout {
                break;
            }
        }

        Ok(num_bytes_read)
    }
}

struct RxBuffer {
    start: u8,
    len: u8,
    data: [u8; INTERRUPT_REPORT_LENGTH],
}

impl RxBuffer {
    fn new() -> Self {
        Self {
            start: 0,
            len: 0,
            data: [0; INTERRUPT_REPORT_LENGTH],
        }
    }

    fn read(&mut self, dest: &mut [u8]) -> usize {
        if self.len == 0 {
            return 0;
        }

        let num_bytes_read = min(dest.len(), self.len as usize);
        let start = self.start as usize;
        let end = start + num_bytes_read;
        let source_buf = &self.data[start..end];
        dest[0..num_bytes_read].copy_from_slice(&source_buf);
        self.len -= num_bytes_read as u8;
        if self.len == 0 {
            self.start = 0;
        } else {
            self.start += num_bytes_read as u8;
        }

        return num_bytes_read;
    }

    fn write(&mut self, source: &[u8]) {
        if source.len() == 0 {
            return;
        }

        self.data[0..source.len()].copy_from_slice(&source);
        self.start = 0;
        self.len = source.len() as u8;
    }

    fn clear(&mut self) {
        self.start = 0;
        self.len = 0;
    }
}
