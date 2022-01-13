#![no_std]
#![no_main]

// Debugging imports
use panic_halt as _;
use rtt_target::{rtt_init_print, rprintln};

// System imports
use cortex_m::prelude::*;
use cortex_m_rt::entry;
//use embedded_hal::
use nrf52832_hal as hal;
use nrf52832_hal::{
    pac,
    gpio::{
        p0,             // access to port0 pins
        Level,
        Output,
        PushPull,
    },
    timer::Timer,
    twim::{self, Twim},
};
use sh1107::{
    prelude::*,
    Builder,
};
use enum_iterator::IntoEnumIterator;
use embedded_graphics::{
    primitives::{Rectangle, PrimitiveStyle},
    mono_font::{ascii::FONT_6X9, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};

#[entry]
fn start() -> ! {
    rtt_init_print!();

    let mut peripherals = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();

    let mut timer = Timer::new(peripherals.TIMER0);

    // random number generator!
    let mut rng = hal::rng::Rng::new(peripherals.RNG);

    // Display setup
    let port0 = p0::Parts::new(peripherals.P0);
    let sda = port0.p0_25.into_floating_input().degrade();
    let scl = port0.p0_26.into_floating_input().degrade();
    let i2c_pins = twim::Pins {scl, sda};
    let i2c = Twim::new(peripherals.TWIM0, i2c_pins, twim::Frequency::K400);
    let disp_size = DisplaySize::Display64x128;
    let mut rotation = DisplayRotation::Rotate270;
    let mut display: GraphicsMode<_> = sh1107::Builder::new()
        .with_size(disp_size)
        .with_rotation(rotation)
        .connect_i2c(i2c)
        .into();

    rprintln!("init display");
    let (DISPLAY_WIDTH, DISPLAY_HEIGHT) = display.get_dimensions();
    rprintln!("display width: {}, display height: {}", DISPLAY_WIDTH, DISPLAY_HEIGHT);
    display.init();

    let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On);
    let mut text = Text::new("hello, rust!", Point::new(1, (DISPLAY_HEIGHT-1) as i32), style);
    let bb: Rectangle = text.bounding_box();
    text.draw(&mut display).unwrap();
    bb.into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1)).draw(&mut display).unwrap();
    display.flush().unwrap();
    timer.delay_ms(2000_u32);
    display.clear();

    rprintln!("loop start");
    const DELAY_MS: u32 = 1;
    let mut loop_counter = (0..10).cycle();
    loop {
        let loop_n = loop_counter.next().unwrap();
        let (t1, t2) = (rng.random_u8(), rng.random_u8());
        let ct1 = map(t1 as i32, 0, 255, 0, (DISPLAY_WIDTH-40) as i32);
        let ct2 = map(t2 as i32, 0, 255, 10, (DISPLAY_HEIGHT-1) as i32);
        text.position.x = ct1;
        text.position.y = ct2;
        text.draw(&mut display).unwrap();
        //text.bounding_box()
        //    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, loop_n))
        //    .draw(&mut display).unwrap();
        display.flush().unwrap();
        display.clear();
        timer.delay_ms(DELAY_MS);
    }

}

// like the Arduino map function
fn map(val: i32, from_low: i32, from_high: i32, to_low: i32, to_high: i32) -> i32 {
    return (val - from_low) * (to_high - to_low) / (from_high - from_low) + to_low;
}
