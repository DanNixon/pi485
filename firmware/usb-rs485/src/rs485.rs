use crate::{Rs485Uart0Resources, Rs485Uart1Resources};
use defmt::{debug, warn};
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::{UART0, UART1},
    uart::{BufferedInterruptHandler, BufferedUart, BufferedUartRx, BufferedUartTx, Config},
};
use embassy_sync::pubsub::WaitResult;
use embedded_io_async::{Read, Write};
use heapless::Vec;
use static_cell::StaticCell;

use super::{RS485_TO_USB, USB_TO_RS485};

bind_interrupts!(struct IrqsUart0 {
    UART0_IRQ  => BufferedInterruptHandler<UART0>;
});

bind_interrupts!(struct IrqsUart1 {
    UART1_IRQ  => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::task]
pub(super) async fn usb_task(spawner: Spawner, r: Rs485Uart0Resources, config: Config) {
    const TX_BUFFER_SIZE: usize = 32;
    const RX_BUFFER_SIZE: usize = 32;

    static TX_BUFFER: StaticCell<[u8; TX_BUFFER_SIZE]> = StaticCell::new();
    let tx_buf = &mut TX_BUFFER.init([0; TX_BUFFER_SIZE])[..];

    static RX_BUFFER: StaticCell<[u8; RX_BUFFER_SIZE]> = StaticCell::new();
    let rx_buf = &mut RX_BUFFER.init([0; RX_BUFFER_SIZE])[..];

    let uart = BufferedUart::new(
        r.uart, r.tx_pin, r.rx_pin, IrqsUart0, tx_buf, rx_buf, config,
    );

    let (tx, rx) = uart.split();

    spawner.must_spawn(tx_task(tx));
    spawner.must_spawn(rx_task(rx));
}

#[embassy_executor::task]
async fn tx_task(mut tx: BufferedUartTx) {
    let mut subscriber = USB_TO_RS485.subscriber().unwrap();

    loop {
        match subscriber.next_message().await {
            WaitResult::Lagged(_) => {
                warn!("Subscriber lagged");
            }
            WaitResult::Message(msg) => {
                if let Err(e) = tx.write(&msg).await {
                    warn!("Failed writing to UART: {}", e);
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn rx_task(mut rx: BufferedUartRx) {
    let publisher = RS485_TO_USB.publisher().unwrap();

    let mut buf = [0u8; 64];

    loop {
        let n = rx.read(&mut buf).await.unwrap();
        debug!("Read {} bytes on UART", n);
        let data = &buf[..n];

        let vec = Vec::from_slice(data).unwrap();
        publisher.publish(vec).await;
    }
}

#[embassy_executor::task]
pub(super) async fn echo_task(r: Rs485Uart1Resources, config: Config) {
    const TX_BUFFER_SIZE: usize = 32;
    const RX_BUFFER_SIZE: usize = 32;

    static TX_BUFFER: StaticCell<[u8; TX_BUFFER_SIZE]> = StaticCell::new();
    let tx_buf = &mut TX_BUFFER.init([0; TX_BUFFER_SIZE])[..];

    static RX_BUFFER: StaticCell<[u8; RX_BUFFER_SIZE]> = StaticCell::new();
    let rx_buf = &mut RX_BUFFER.init([0; RX_BUFFER_SIZE])[..];

    let mut uart = BufferedUart::new(
        r.uart, r.tx_pin, r.rx_pin, IrqsUart1, tx_buf, rx_buf, config,
    );

    let mut buf = [0u8; 64];

    loop {
        let n = uart.read(&mut buf).await.unwrap();
        debug!("Read {} bytes on UART", n);
        let data = &buf[..n];
        uart.write(data).await.unwrap();
    }
}
