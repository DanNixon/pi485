use crate::{Rs485Uart0Resources, Rs485Uart1Resources};
use defmt::info;
use embassy_futures::select::{select3, Either3};
use embassy_rp::{
    bind_interrupts,
    peripherals::{UART0, UART1},
    uart::{BufferedInterruptHandler, BufferedUart, Config},
};
use embassy_time::{Duration, Ticker};
use embedded_io_async::{Read, Write};
use static_cell::StaticCell;

bind_interrupts!(struct IrqsUart0 {
    UART0_IRQ  => BufferedInterruptHandler<UART0>;
});

bind_interrupts!(struct IrqsUart1 {
    UART1_IRQ  => BufferedInterruptHandler<UART1>;
});

#[embassy_executor::task]
pub(super) async fn task(r0: Rs485Uart0Resources, r1: Rs485Uart1Resources) {
    let mut config = Config::default();
    config.baudrate = 115200;

    const TX_BUFFER_SIZE: usize = 32;
    const RX_BUFFER_SIZE: usize = 32;

    static TX_BUFFER_0: StaticCell<[u8; TX_BUFFER_SIZE]> = StaticCell::new();
    let tx_buf_0 = &mut TX_BUFFER_0.init([0; TX_BUFFER_SIZE])[..];

    static RX_BUFFER_0: StaticCell<[u8; RX_BUFFER_SIZE]> = StaticCell::new();
    let rx_buf_0 = &mut RX_BUFFER_0.init([0; RX_BUFFER_SIZE])[..];

    static TX_BUFFER_1: StaticCell<[u8; TX_BUFFER_SIZE]> = StaticCell::new();
    let tx_buf_1 = &mut TX_BUFFER_1.init([0; TX_BUFFER_SIZE])[..];

    static RX_BUFFER_1: StaticCell<[u8; RX_BUFFER_SIZE]> = StaticCell::new();
    let rx_buf_1 = &mut RX_BUFFER_1.init([0; RX_BUFFER_SIZE])[..];

    let uart0 = BufferedUart::new(
        r0.uart, r0.tx_pin, r0.rx_pin, IrqsUart0, tx_buf_0, rx_buf_0, config,
    );

    let uart1 = BufferedUart::new(
        r1.uart, r1.tx_pin, r1.rx_pin, IrqsUart1, tx_buf_1, rx_buf_1, config,
    );

    let (mut tx0, mut rx0) = uart0.split();
    let (mut tx1, mut rx1) = uart1.split();

    let mut tx_tick = Ticker::every(Duration::from_secs(1));

    let mut rx0_buff = [0u8; 64];
    let mut rx1_buff = [0u8; 64];

    loop {
        match select3(
            tx_tick.next(),
            rx0.read(&mut rx0_buff),
            rx1.read(&mut rx1_buff),
        )
        .await
        {
            Either3::First(_) => {
                tx0.write_all(b"Hello from UART 0").await.unwrap();
                tx1.write(b"Hello from UART 1").await.unwrap();
            }
            Either3::Second(res) => {
                info!("UART 0 rx: {}", res);
            }
            Either3::Third(res) => {
                info!("UART 1 rx: {}", res);
            }
        }
    }
}
