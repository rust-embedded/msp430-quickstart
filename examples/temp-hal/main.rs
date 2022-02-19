//! Temperature sensor demo for the [MSP-EXP430G2](http://www.ti.com/tool/MSP-EXP430G2)
//! development kit. Make sure jumpers are set to HW UART, (possibly) disconnect the green LED
//! jumper, and attach a [TCN75A](https://www.microchip.com/en-us/product/TCN75A) to pins 1.6
//! (SCK) and 1.7 (SDA). Push the button attached to 1.3 to toggle between F, and C!
//!
//! ---

#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

mod hal;
use hal::*;

mod newtypes;

extern crate panic_msp430;

use core::cell::{Cell, RefCell};
use core::fmt::Write;

use embedded_hal::serial::{self, nb::Write as SerWrite};
use embedded_hal::timer::nb::CountDown;
use embedded_hal::watchdog::blocking::Disable;
use fixed::traits::LossyFrom;
use fixed::types::{I8F8, I9F7};
use fixed_macro::types::{I8F8, I9F7};
use msp430::interrupt as mspint;
use msp430_rt::entry;
use {{device}}::{interrupt, Peripherals};
use once_cell::unsync::OnceCell;
use tcn75a::{ConfigReg, Resolution, Tcn75a};

// Serial in the future may use interrupts. TCN75A is currently blocking and does not use
// interrupts, thus is not static.
static TIMER: mspint::Mutex<RefCell<Option<Timer>>> = mspint::Mutex::new(RefCell::new(None));
static SERIAL: mspint::Mutex<RefCell<Option<Serial>>> = mspint::Mutex::new(RefCell::new(None));
static PORT1_PINS: mspint::Mutex<OnceCell<{{device}}::PORT_1_2>> = mspint::Mutex::new(OnceCell::new());
static TEMP_DISPLAY: mspint::Mutex<Cell<TempDisplay>> = mspint::Mutex::new(Cell::new(TempDisplay::Celsius));

#[derive(Debug, Clone, Copy)]
enum TempDisplay {
    Celsius,
    Fahrenheit,
}

fn init(cs: mspint::CriticalSection) -> Tcn75a<I2c> {
    let p = Peripherals::take().unwrap();

    WatchdogTimer::new(p.WATCHDOG_TIMER).disable().unwrap();

    let clock = &p.SYSTEM_CLOCK; // Default clk is around 1.1 MHz using DCO. Submain clock also fed by it.
    clock.bcsctl3.modify(|_, w| w.lfxt1s().lfxt1s_2()); // Use internal VLO for AUX clock (12kHz).
    clock.bcsctl1.modify(|_, w| w.diva().diva_1()); // Divide AUX clock by two (6000 Hz).

    let port_1_2 = &p.PORT_1_2;
    port_1_2.p1dir.modify(|_, w| w.p0().set_bit().p3().clear_bit());
    port_1_2.p1out.modify(|_, w| w.p0().set_bit().p3().set_bit()); // Pullup on P3.
    port_1_2.p1ren.modify(|_, w| w.p3().set_bit());

    // Set bits for UART and I2C operation.
    port_1_2.p1sel.modify(|_, w| {
        w.p1()
            .set_bit()
            .p2()
            .set_bit()
            .p6()
            .set_bit()
            .p7()
            .set_bit()
    });
    port_1_2.p1sel2.modify(|_, w| {
        w.p1()
            .set_bit()
            .p2()
            .set_bit()
            .p6()
            .set_bit()
            .p7()
            .set_bit()
    });

    // Set bit to interrupt on button on P1.3
    port_1_2.p1ie.modify(|_, w| {
        w.p3().set_bit()
    });

    let mut timer = Timer::new(p.TIMER0_A3);
    timer.start(6000u16).unwrap();

    let serial = Serial::new(p.USCI_A0_UART_MODE);

    let i2c_flags = SfrIfg::new(p.SPECIAL_FUNCTION);
    let i2c = I2c::new(p.USCI_B0_I2C_MODE, i2c_flags.ucb0ifg);

    let mut tcn = Tcn75a::new(i2c, 0x48);
    let mut cfg = ConfigReg::new();
    cfg.set_resolution(Resolution::Bits12);
    tcn.set_config_reg(cfg).unwrap();

    *TIMER.borrow(cs).borrow_mut() = Some(timer);
    *SERIAL.borrow(cs).borrow_mut() = Some(serial);
    PORT1_PINS.borrow(cs).set(p.PORT_1_2).ok().unwrap();

    tcn
}

#[entry(interrupt_enable(pre_interrupt = init))]
fn main(mut tcn: Tcn75a<I2c>) -> ! {
    loop {
        mspint::free(|cs| {
            let mut t_ref = TIMER.borrow(*cs).borrow_mut();
            let mut s_ref = SERIAL.borrow(*cs).borrow_mut();

            match t_ref.as_mut().unwrap().wait() {
                Ok(()) => {
                    let tmp_result = tcn.temperature();

                    // Avoid bringing in formatting for panic due to optimization
                    // issues.
                    let tmp: I8F8 = match tmp_result {
                        Ok(t) => { t.into() }
                        Err(_) => { I8F8!(0) }
                    };

                    let s: &mut dyn SerWrite<Error = serial::ErrorKind> = s_ref.as_mut().unwrap();

                    match TEMP_DISPLAY.borrow(*cs).get() {
                        TempDisplay::Celsius => {
                            let tmp_c: newtypes::fmt::I8F8SmallFmt = tmp.into();
                            write!(s, "{} C\n", tmp_c).unwrap()
                        }
                        TempDisplay::Fahrenheit => {
                            // Don't bring in FixedI32 formatting.
                            let tmp_f: newtypes::fmt::I9F7SmallFmt = (I9F7!(1.8) * I9F7::lossy_from(tmp) + I9F7!(32)).into();
                            write!(s, "{} F\n", tmp_f).unwrap()
                        },
                    }
                }
                _ => {}
            }
        })
    }
}

#[interrupt]
fn PORT1(cs: CriticalSection) {
    let p = PORT1_PINS.borrow(cs).get().unwrap();
    let mut temp_display = TEMP_DISPLAY.borrow(cs).get();

    temp_display = match temp_display {
        TempDisplay::Celsius => TempDisplay::Fahrenheit,
        TempDisplay::Fahrenheit => TempDisplay::Celsius,
    };

    TEMP_DISPLAY.borrow(cs).set(temp_display);

    p.p1ifg.modify(|_, w| {
        w.p3().clear_bit()
    });
}

#[interrupt]
fn TIMER0_A1(cs: CriticalSection) {
    // let p = PERIPHERALS.borrow(cs).get().unwrap();
    let mut t_ref = TIMER.borrow(cs).borrow_mut();

    let t = t_ref.as_mut().unwrap();
    t.timer_int();

    // let port_1_2 = &p.PORT_1_2;
    // port_1_2.p1out.modify(|r, w| w.p0().bit(!r.p0().bit()));
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
