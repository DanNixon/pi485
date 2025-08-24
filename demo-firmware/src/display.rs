use crate::DisplayResources;
use core::{cell::RefCell, convert::Infallible};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{
    gpio::{Level, Output},
    pwm::{Pwm, SetDutyCycle},
};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::{Delay, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{DrawTarget, Primitive, WebColors},
    primitives::{Line, PrimitiveStyleBuilder, StrokeAlignment},
    text::{Alignment, Text},
    Drawable,
};
use embedded_hal::digital::{ErrorType, OutputPin};
use mipidsi::{interface::SpiInterface, models::ST7789, options::ColorInversion, Builder};

#[embassy_executor::task]
pub(super) async fn task(r: DisplayResources) {
    let mut config = embassy_rp::spi::Config::default();
    config.frequency = 64_000_000;
    config.phase = embassy_rp::spi::Phase::CaptureOnSecondTransition;
    config.polarity = embassy_rp::spi::Polarity::IdleHigh;

    let spi =
        embassy_rp::spi::Spi::new_blocking_txonly(r.spi, r.clk_pin, r.mosi_pin, config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let display_spi = SpiDeviceWithConfig::new(&spi_bus, NoCs, config);

    let mut backlight = Pwm::new_output_a(
        r.backlight_pwm,
        r.backlight_pin,
        embassy_rp::pwm::Config::default(),
    );

    let dc = Output::new(r.dc_pin, Level::Low);
    let rst = Output::new(r.reset_pin, Level::Low);

    let mut buffer = [0_u8; 512];
    let interface = SpiInterface::new(display_spi, dc, &mut buffer);

    let mut display = Builder::new(ST7789, interface)
        .display_size(240, 240)
        .invert_colors(ColorInversion::Inverted)
        .reset_pin(rst)
        .init(&mut Delay)
        .unwrap();

    BootScreen {}.draw(&mut display).unwrap();

    let _ = backlight.set_duty_cycle_fraction(0, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(1, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(2, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(3, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(4, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(5, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(6, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(7, 8);
    Timer::after_millis(250).await;
    let _ = backlight.set_duty_cycle_fraction(8, 8);
    Timer::after_millis(250).await;

    loop {
        Timer::after_secs(10).await;
    }
}

struct NoCs;

impl OutputPin for NoCs {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl ErrorType for NoCs {
    type Error = Infallible;
}

pub(crate) struct BootScreen {}

impl Drawable for BootScreen {
    type Output = ();
    type Color = Rgb565;

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let text_style = MonoTextStyle::new(&FONT_10X20, Self::Color::CSS_BLACK);

        let line_style = PrimitiveStyleBuilder::new()
            .stroke_color(Self::Color::CSS_YELLOW)
            .stroke_width(1)
            .stroke_alignment(StrokeAlignment::Inside)
            .build();

        let display_box = target.bounding_box();

        // Fill the display with pink pixels
        target.clear(Self::Color::CSS_HOT_PINK)?;

        // Draw a one pixel border around the display
        display_box.into_styled(line_style).draw(target)?;

        // Draw a line through the display area
        Line::new(
            display_box
                .bottom_right()
                .expect("the display bounding box should be of size > 1x1"),
            display_box.top_left,
        )
        .into_styled(line_style)
        .draw(target)?;

        // Show some text
        Text::with_alignment(
            "pi485\nDemo Firmware",
            display_box.center(),
            text_style,
            Alignment::Center,
        )
        .draw(target)?;

        Ok(())
    }
}
