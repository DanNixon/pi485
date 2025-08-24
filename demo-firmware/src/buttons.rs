use crate::ButtonResources;
use defmt::info;
use embassy_futures::select::{select3, Either3};
use embassy_rp::gpio::{Input, Pull};
use embassy_time::Timer;

#[embassy_executor::task]
pub(super) async fn task(r: ButtonResources) {
    let mut a = Input::new(r.a_pin, Pull::Up);
    let mut b = Input::new(r.b_pin, Pull::Up);
    let mut c = Input::new(r.c_pin, Pull::Up);

    loop {
        match select3(
            a.wait_for_falling_edge(),
            b.wait_for_falling_edge(),
            c.wait_for_falling_edge(),
        )
        .await
        {
            Either3::First(_) => info!("A pressed"),
            Either3::Second(_) => info!("B pressed"),
            Either3::Third(_) => info!("C pressed"),
        }

        Timer::after_millis(250).await;
    }
}
