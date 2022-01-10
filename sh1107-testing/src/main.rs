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
    pac::{TWIM0},
    gpio::{
        p0,             // access to port0 pins
        Level,
        Output,
        PushPull,
    },
    twim::{self, Twim},
};
use sh1107::{
    prelude::*,
    Builder,
};

#[entry]
fn start() -> ! {
    rtt_init_print!();

    let mut peripherals = nrf52832_hal::pac::Peripherals::take().unwrap();
    let mut core = nrf52832_hal::pac::CorePeripherals::take().unwrap();

    // Display setup
    let port0 = p0::Parts::new(peripherals.P0);
    let sda = port0.p0_25.into_floating_input().degrade();
    let scl = port0.p0_26.into_floating_input().degrade();
    let i2c_pins = twim::Pins {scl, sda};
    let i2c = Twim::new(peripherals.TWIM0, i2c_pins, twim::Frequency::K250);
    let disp_size = DisplaySize::Display64x128;
    let mut display: GraphicsMode<_> = Builder::new().with_size(disp_size).connect_i2c(i2c).into();
    //let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();

    let (x, y) = display.get_dimensions();
    rprintln!("display size: {}, {}", x, y);
    rprintln!("init display");
    display.init();
    //display.clear();
    display.set_pixel(20, 2, 1);
    for x in 0..20 {
        for y in 0..20 {
            display.set_pixel(20+x, 20+y, 1);
        }
    }

    display.set_pixel(2, 20, 1);
    display.set_pixel(3, 20, 1);

    display.flush().unwrap();

    loop {}

}

