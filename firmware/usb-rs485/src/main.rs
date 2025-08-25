#![no_std]
#![no_main]

mod rs485;
mod usb;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    peripherals,
    uart::{Config, DataBits, Parity, StopBits},
    Peri,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pubsub::PubSubChannel};
use heapless::Vec;
use panic_probe as _;
use portable_atomic as _;

assign_resources::assign_resources! {
    rs485_uart_0: Rs485Uart0Resources {
        tx_pin: PIN_0,
        rx_pin: PIN_1,
        uart: UART0,
    },
    rs485_uart_1: Rs485Uart1Resources {
        tx_pin: PIN_4,
        rx_pin: PIN_5,
        uart: UART1,
    },
    usb: UsbResources {
        usb: USB,
    },
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);

    info!("Hello, world!");

    let uart_config = {
        let mut config = Config::default();
        config.baudrate = 19200;
        config.data_bits = DataBits::DataBits8;
        config.parity = Parity::ParityNone;
        config.stop_bits = StopBits::STOP1;
        config
    };

    spawner.must_spawn(usb::task(spawner, r.usb));
    spawner.must_spawn(rs485::usb_task(spawner, r.rs485_uart_0, uart_config));
    spawner.must_spawn(rs485::echo_task(r.rs485_uart_1, uart_config));
}

pub(crate) type Payload = Vec<u8, 64>;

pub(crate) static USB_TO_RS485: PubSubChannel<CriticalSectionRawMutex, Payload, 8, 1, 1> =
    PubSubChannel::new();
pub(crate) static RS485_TO_USB: PubSubChannel<CriticalSectionRawMutex, Payload, 8, 1, 1> =
    PubSubChannel::new();
