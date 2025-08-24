use crate::{EthernetResources, SharedSpi, SharedSpiInner};
use defmt::{info, unwrap};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_futures::yield_now;
use embassy_net::{Stack, StackResources};
use embassy_net_wiznet::{chip::W5500, Device, Runner, State};
use embassy_rp::{
    clocks::RoscRng,
    gpio::{Input, Level, Output, Pull},
    spi::Config,
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Timer;
use static_cell::StaticCell;

#[embassy_executor::task]
pub(super) async fn task(spawner: Spawner, spi: &'static SharedSpi, r: EthernetResources) {
    let mut rng = RoscRng;

    let mut config = Config::default();
    config.frequency = 50_000_000;
    config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
    config.polarity = embassy_rp::spi::Polarity::IdleHigh;

    let cs = Output::new(r.cs_pin, Level::High);
    let device = SpiDeviceWithConfig::new(spi, cs, config);

    let w5500_int = Input::new(r.int_pin, Pull::Up);
    let w5500_reset = Output::new(r.rst_pin, Level::High);

    let mac_addr = [0x02, 0x00, 0x00, 0x00, 0x00, 0x00];

    static STATE: StaticCell<State<8, 8>> = StaticCell::new();
    let state = STATE.init(State::<8, 8>::new());

    let (device, runner) = embassy_net_wiznet::new(mac_addr, state, device, w5500_int, w5500_reset)
        .await
        .unwrap();

    unwrap!(spawner.spawn(ethernet_task(runner)));

    let seed = rng.next_u64();

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

    let (stack, runner) = embassy_net::new(
        device,
        embassy_net::Config::dhcpv4(Default::default()),
        RESOURCES.init(StackResources::new()),
        seed,
    );

    unwrap!(spawner.spawn(net_task(runner)));

    info!("Waiting for DHCP...");
    let cfg = wait_for_config(stack).await;
    let local_addr = cfg.address.address();
    info!("IP address: {:?}", local_addr);

    loop {
        Timer::after_secs(10).await;
    }
}

#[embassy_executor::task]
async fn ethernet_task(
    runner: Runner<
        'static,
        W5500,
        SpiDeviceWithConfig<'static, CriticalSectionRawMutex, SharedSpiInner, Output<'static>>,
        Input<'static>,
        Output<'static>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device<'static>>) -> ! {
    runner.run().await
}

async fn wait_for_config(stack: Stack<'static>) -> embassy_net::StaticConfigV4 {
    loop {
        if let Some(config) = stack.config_v4() {
            return config.clone();
        }
        yield_now().await;
    }
}
