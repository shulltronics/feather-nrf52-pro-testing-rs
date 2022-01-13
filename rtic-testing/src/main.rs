#![no_std]
#![no_main]

use rtic::app;
use panic_halt as _;
mod monotonic_timer0;

#[app(device = nrf52832_hal::pac, peripherals = true, dispatchers = [RTC2])]
mod app {

    use nrf52832_hal as hal;
    use nrf52832_hal::{
        pac::{TIMER0, TWIM0, SPI1},
        gpio::{
            Level,
            Output,
            PushPull,
        },
        twim::{self, Twim},     // "Two-wire interface master"
        spi::{self, Spi},     // "Serial peripheral interface"
    };
    use embedded_hal::digital::v2::OutputPin;
    use rtt_target::{rtt_init_print, rprintln};
    use super::monotonic_timer0::{MonoTimer, ExtU32};
    use sh1107::{prelude::*, Builder};
    use smart_leds::{SmartLedsWrite, RGB8};
    use ws2812_spi::{MODE as NeoPixel_SPI_MODE, Ws2812};

    #[monotonic(binds = TIMER0, default = true)]
    type MyMono = MonoTimer<TIMER0>;

    #[shared]
    struct DataCommon {
    }

    #[local]
    struct DataLocal {
        led: hal::gpio::p0::P0_17<Output<PushPull>>,
        state: bool,
        display: GraphicsMode<I2cInterface<hal::twim::Twim<TWIM0>>>,
        neopixel: Ws2812<hal::spi::Spi<SPI1>>,
    }

    #[init]
    fn init(cx: init::Context) -> (DataCommon, DataLocal, init::Monotonics) {
        rtt_init_print!();
        rprintln!("entered init...");

        let peripherals: nrf52832_hal::pac::Peripherals = cx.device;
        let core = cx.core;
        rprintln!("got peripherals");

        let mono = MonoTimer::new(peripherals.TIMER0);

        // setup GPIO
        let port0 = hal::gpio::p0::Parts::new(peripherals.P0);
        // setup I2C and OLED display
        let scl = port0.p0_26.into_floating_input().degrade();
        let sda = port0.p0_25.into_floating_input().degrade();
        let i2c_pins = twim::Pins {scl, sda};
        let i2c = Twim::new(peripherals.TWIM0, i2c_pins, twim::Frequency::K100);
        let display_size = DisplaySize::Display64x128;
        let mut display: GraphicsMode<_> = Builder::new().with_size(display_size).connect_i2c(i2c).into();
        // setup SPI and neopixel
        let sck = port0.p0_12.into_push_pull_output(Level::Low).degrade();
        let mosi = port0.p0_13.into_push_pull_output(Level::Low).degrade();
        let spi_pins = spi::Pins {sck, mosi: Some(mosi), miso: None};
        let spi = Spi::new(
            peripherals.SPI1,
            spi_pins,
            spi::Frequency::M2,
            NeoPixel_SPI_MODE
        );
        let mut neopixel = Ws2812::new(spi);
        let pixels = [RGB8::new(0, 0, 0)];
        neopixel.write(pixels.iter().cloned());

        rprintln!("init display...\n");
        display.init().unwrap();
        rprintln!("write pixel...\n");
        display.set_pixel(1,1,1);
        display.flush().unwrap();

        let mut led_pin = port0.p0_17.into_push_pull_output(Level::Low);
        led_pin.set_high().unwrap();

        rprintln!("Spawning blink task...\n");
        blink::spawn().unwrap();
        let ms = 1_500_000.micros();
        rprintln!("scheduling screen_clear() for {} microseconds in the future", ms);
        screen_clear::spawn_after(ms).unwrap();

        (
            DataCommon {},
             DataLocal {
                led: led_pin,
                state: false,
                display: display,
                neopixel: neopixel,
             },
             init::Monotonics(mono)
        )
    } 

    #[task(local = [led, state])]
    fn blink(cx: blink::Context) {
        rprintln!("blink!");
        let v = !*cx.local.state;
        match v {
            true =>  cx.local.led.set_high().unwrap(),
            false => cx.local.led.set_low().unwrap(),
        }
        *cx.local.state = v;
        rprintln!("Value of cx.local.state: {}", cx.local.state);
        blink::spawn_after(1_000_000.micros()).unwrap();
    }

    #[task(local = [display])]
    fn screen_clear(cx: screen_clear::Context) {
        rprintln!("screen_clear!");
        let mut d = cx.local.display;
        d.clear();
        d.flush().unwrap();
        rprintln!("leaving screen_clear..");
    }

}
