use crate::{UsbResources, RS485_TO_USB, USB_TO_RS485};
use defmt::{debug, info, warn};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_rp::{
    bind_interrupts,
    peripherals::USB,
    usb::{Driver, Instance, InterruptHandler},
};
use embassy_sync::pubsub::WaitResult;
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
    Config, UsbDevice,
};
use heapless::Vec;
use static_cell::StaticCell;

#[embassy_executor::task]
pub(super) async fn task(spawner: Spawner, r: UsbResources) {
    let usb_driver = Driver::new(r.usb, Irqs);

    let usb_config = {
        let mut config = Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("dannixon");
        config.product = Some("USB-RS485 on pi485");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };

    let mut usb_builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        embassy_usb::Builder::new(
            usb_driver,
            usb_config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        )
    };

    let mut usb_class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut usb_builder, state, 64)
    };

    let usb = usb_builder.build();

    spawner.must_spawn(usb_task(usb));

    loop {
        usb_class.wait_connection().await;
        info!("Connected");
        let _ = echo(&mut usb_class).await;
        info!("Disconnected");
    }
}

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

type MyUsbDriver = Driver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;

#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn echo<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    let publisher = USB_TO_RS485.publisher().unwrap();
    let mut subscriber = RS485_TO_USB.subscriber().unwrap();

    let mut buf = [0; 64];

    loop {
        match select(class.read_packet(&mut buf), subscriber.next_message()).await {
            Either::First(n) => {
                let n = n.unwrap();
                debug!("Read {} bytes on UART", n);

                let data = Vec::from_slice(&buf[..n]).unwrap();
                publisher.publish(data).await;
            }
            Either::Second(msg) => match msg {
                WaitResult::Lagged(_) => {
                    warn!("Subscriber lagged");
                }
                WaitResult::Message(data) => {
                    class.write_packet(&data).await?;
                }
            },
        }
    }
}
