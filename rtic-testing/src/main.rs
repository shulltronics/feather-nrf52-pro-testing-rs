#![no_std]
#![no_main]

use rtic::app;
//use panic_rtt_target as _;
use panic_halt as _;
mod monotonic_timer0;

#[app(device = nrf52832_hal::pac, peripherals = true, dispatchers = [RTC2])]
mod app {

    use nrf52832_hal as hal;
    use nrf52832_hal::{
        pac::{TIMER0, TWIM0, PWM1},
        gpio::{
            Level,
            Output,
            PushPull,
        },
        clocks::{self, Clocks},
        twim::{self, Twim},     // "Two-wire interface master"
        spi::{self, Spi},       // "Serial peripheral interface"
        pwm::{self, Pwm},       // Pulse Width Modulation interface"
        time::{Hertz},
    };
    use embedded_hal::{
        Pwm as EHPwm,
        digital::v2::OutputPin,
    };
    use rtt_target::{rtt_init_print, rprintln};
    use super::monotonic_timer0::{MonoTimer, ExtU32};
    use sh1107::{prelude::*, Builder};
    use embedded_graphics::{
        primitives::{Rectangle, PrimitiveStyle},
        mono_font::{ascii::FONT_6X9, MonoTextStyle},
        pixelcolor::BinaryColor,
        prelude::*,
        text::Text,
    };
    use smart_leds::{SmartLedsWrite, RGB8};
    use ws2812_spi::{MODE as NeoPixel_SPI_MODE, Ws2812};

    #[monotonic(binds = TIMER0, default = true)]
    type MyMono = MonoTimer<TIMER0>;

    #[shared]
    struct DataCommon {
    }

    #[local]
    struct DataLocal {
        led: hal::gpio::p0::P0_19<Output<PushPull>>,
        state: bool,
        display: GraphicsMode<I2cInterface<hal::twim::Twim<TWIM0>>>,
        //neopixel: Ws2812<hal::spi::Spi<SPI1>>,
        pwm: Pwm<PWM1>,
        val: u16,
        dir: bool,
    }

    #[init]
    fn init(cx: init::Context) -> (DataCommon, DataLocal, init::Monotonics) {
        //rtt_init_print!();
        //rprintln!("entered init...");

        let peripherals: nrf52832_hal::pac::Peripherals = cx.device;
        let core = cx.core;
        let _clocks = Clocks::new(peripherals.CLOCK).enable_ext_hfosc();
        //rprintln!("got peripherals");

        let mono = MonoTimer::new(peripherals.TIMER0);

        // setup GPIO
        let port0 = hal::gpio::p0::Parts::new(peripherals.P0);

        // setup PWM
        let pwm_pin = port0.p0_17.into_push_pull_output(Level::Low).degrade();
        let mut pwm = Pwm::new(peripherals.PWM1);
        // get metrics
        let max_duty = pwm.get_max_duty();
        //rprintln!("pwm max duty: {}", max_duty);

        pwm.set_period(Hertz(500_u32))
            .set_output_pin(pwm::Channel::C0, pwm_pin);
        pwm.enable();
        pwm.set_duty_on(pwm::Channel::C0, 0);
        //rprintln!("pwm init duty cycle: {}", pwm.duty_on(pwm::Channel::C0));

        // setup I2C and OLED display
        let scl = port0.p0_26.into_floating_input().degrade();
        let sda = port0.p0_25.into_floating_input().degrade();
        let i2c_pins = twim::Pins {scl, sda};
        let i2c = Twim::new(peripherals.TWIM0, i2c_pins, twim::Frequency::K400);
        let display_size = DisplaySize::Display64x128;
        let display_rot  = DisplayRotation::Rotate270;
        let mut display: GraphicsMode<_> = Builder::new()
            .with_size(display_size)
            .with_rotation(display_rot)
            .connect_i2c(i2c)
            .into();
        /* setup SPI and neopixel
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
        */

        //rprintln!("init display...\n");
        display.init().unwrap();
        // define header text
        let style = MonoTextStyle::new(&FONT_6X9, BinaryColor::On);
        let mut text = Text::new("RTIC testing", Point::new(3, 0), style);
        // get it's size and shift it down appropriately
        let bb: Rectangle = text.bounding_box();
        let (tw, th) = (bb.size.width as i32, bb.size.height as i32);
        text.position.y = th + 3;
        text.draw(&mut display);
        display.flush().unwrap();

        let mut led_pin = port0.p0_19.into_push_pull_output(Level::Low);
        led_pin.set_high().unwrap();

        //rprintln!("Spawning blink task...\n");
        blink::spawn().unwrap();

        //rprintln!("Spawning pwm_change task...\n");
        pwm_change::spawn().unwrap();

        // return resources
        (
            DataCommon {},
             DataLocal {
                led: led_pin,
                state: false,
                display: display,
                //neopixel: neopixel,
                pwm: pwm,
                val: 0,
                dir: true
             },
             init::Monotonics(mono)
        )
    } 

    #[task(local = [led, state])]
    fn blink(cx: blink::Context) {
        ////rprintln!("blink!");
        let v = !*cx.local.state;
        match v {
            true =>  cx.local.led.set_high().unwrap(),
            false => cx.local.led.set_low().unwrap(),
        }
        *cx.local.state = v;
        ////rprintln!("Value of cx.local.state: {}", cx.local.state);
        blink::spawn_after(1_000_000.micros()).unwrap();
    }

    #[task(local = [pwm, val, dir])]
    fn pwm_change(cx: pwm_change::Context) {
        let pwm = cx.local.pwm;
        // get current val and check it for out-of-bounds
        let mut val = *cx.local.val;
        let mut dir = *cx.local.dir;
        if val >= pwm.get_max_duty() {
            dir = false;
        } else if val <= 0 {
            dir = true;
        }
        let delta: i16 = match dir {
            true  =>  0xF,
            false => -0xF,
        };
        val = ((val as i16) + delta) as u16;
        *cx.local.val = val;
        *cx.local.dir = dir;
        pwm.set_duty_on(pwm::Channel::C0, val);
        ////rprintln!("pwm val:      {}", val);
        ////rprintln!("pwm duty on:  {}", pwm.duty_on(pwm::Channel::C0));
        ////rprintln!("pwm duty off: {}", pwm.duty_off(pwm::Channel::C0));
        pwm_change::spawn_after(700.micros()).unwrap();
    }

    #[task(local = [display])]
    fn screen_clear(cx: screen_clear::Context) {
        //rprintln!("screen_clear!");
        let mut d = cx.local.display;
        d.clear();
        d.flush().unwrap();
        //rprintln!("leaving screen_clear..");
    }

}
