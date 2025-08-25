#![no_std]
#![no_main]

mod buttons;
mod display;
mod ethernet;
mod rs485;

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    peripherals::{self},
    spi::Spi,
    Peri,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use panic_probe as _;
use portable_atomic as _;
use static_cell::StaticCell;

assign_resources::assign_resources! {
    shared_spi: SharedSpiResources {
        miso: PIN_16,
        mosi: PIN_19,
        clk: PIN_18,
        spi: SPI0,
        tx_dma: DMA_CH0,
        rx_dma: DMA_CH1,
    },
    ethernet: EthernetResources {
        cs_pin: PIN_17,
        int_pin: PIN_21,
        rst_pin: PIN_20,
    },
    sd: SdResources {
        cs_pin: PIN_22,
    },
    display: DisplayResources {
        mosi_pin: PIN_11,
        clk_pin: PIN_10,
        dc_pin: PIN_13,
        reset_pin: PIN_12,
        backlight_pin: PIN_14,
        spi: SPI1,
        backlight_pwm: PWM_SLICE7,
    }
    rs485_uart_0: Rs485Uart0Resources {
        tx_pin: PIN_0,
        rx_pin: PIN_1,
        uart: UART0,
    }
    rs485_uart_1: Rs485Uart1Resources {
        tx_pin: PIN_4,
        rx_pin: PIN_5,
        uart: UART1,
        tx_dma: DMA_CH2,
        rx_dma: DMA_CH3,
    }
    buttons: ButtonResources {
        a_pin: PIN_6,
        b_pin: PIN_7,
        c_pin: PIN_8,
    }
}

type SharedSpiInner = Spi<'static, peripherals::SPI0, embassy_rp::spi::Async>;
type SharedSpi = Mutex<CriticalSectionRawMutex, SharedSpiInner>;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let r = split_resources!(p);

    info!("Hello, world!");

    let mut spi_config = embassy_rp::spi::Config::default();
    spi_config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
    spi_config.polarity = embassy_rp::spi::Polarity::IdleHigh;

    let spi = Spi::new(
        r.shared_spi.spi,
        r.shared_spi.clk,
        r.shared_spi.mosi,
        r.shared_spi.miso,
        r.shared_spi.tx_dma,
        r.shared_spi.rx_dma,
        spi_config,
    );

    let spi: SharedSpi = Mutex::new(spi);

    static SPI_BUS: StaticCell<SharedSpi> = StaticCell::new();
    let spi = SPI_BUS.init(spi);

    spawner.must_spawn(buttons::task(r.buttons));
    spawner.must_spawn(display::task(r.display));
    spawner.must_spawn(ethernet::task(spawner, spi, r.ethernet));
    spawner.must_spawn(rs485::task(r.rs485_uart_0, r.rs485_uart_1));
}
