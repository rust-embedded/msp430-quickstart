#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

mod hal;
use hal::*;

extern crate panic_msp430;

use core::cell::RefCell;
use core::fmt::Write;

use embedded_hal::serial::{self, nb::Write as SerWrite};
use embedded_hal::timer::nb::CountDown;
use embedded_hal::watchdog::blocking::Disable;
use fixed::types::I8F8;
use fixed_macro::types::I8F8;
use msp430::interrupt as mspint;
use msp430_rt::entry;
use {{device}}::{interrupt, Peripherals};
use tcn75a::{ConfigReg, Resolution, Tcn75a};

// static PERIPHERALS : mspint::Mutex<OnceCell<Peripherals>> =
//     mspint::Mutex::new(OnceCell::new());

// Serial in the future may use interrupts. TCN75A is currently blocking and does not use
// interrupts, thus is not static.
static TIMER: mspint::Mutex<RefCell<Option<Timer>>> = mspint::Mutex::new(RefCell::new(None));
static SERIAL: mspint::Mutex<RefCell<Option<Serial>>> = mspint::Mutex::new(RefCell::new(None));

#[entry]
fn main(cs: CriticalSection) -> ! {
    let p = Peripherals::take().unwrap();

    WatchdogTimer::new(p.WATCHDOG_TIMER).disable().unwrap();

    let clock = &p.SYSTEM_CLOCK; // Default clk is around 1.1 MHz using DCO. Submain clock also fed by it.
    clock.bcsctl3.modify(|_, w| w.lfxt1s().lfxt1s_2()); // Use internal VLO for AUX clock (12kHz).
    clock.bcsctl1.modify(|_, w| w.diva().diva_1()); // Divide AUX clock by two (6000 Hz).

    let port_1_2 = &p.PORT_1_2;
    port_1_2.p1dir.modify(|_, w| w.p0().set_bit());
    port_1_2.p1out.modify(|_, w| w.p0().set_bit());

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

    let mut timer = Timer::new(p.TIMER0_A3);
    timer.start(6000u16).unwrap();

    let mut serial = Serial::new(p.USCI_A0_UART_MODE);

    let i2c_flags = SfrIfg::new(p.SPECIAL_FUNCTION);
    let i2c = I2c::new(p.USCI_B0_I2C_MODE, i2c_flags.ucb0ifg);

    let mut tcn = Tcn75a::new(i2c, 0x48);
    let mut cfg = ConfigReg::new();
    cfg.set_resolution(Resolution::Bits12);
    tcn.set_config_reg(cfg).unwrap();

    *TIMER.borrow(cs).borrow_mut() = Some(timer);
    *SERIAL.borrow(cs).borrow_mut() = Some(serial);

    // Safe because interrupts are disabled after a reset.
    unsafe {
        mspint::enable();
    }

    let mut cnt: u8 = 0;
    loop {
        mspint::free(|cs| {
            let mut t_ref = TIMER.borrow(*cs).borrow_mut();
            let mut s_ref = SERIAL.borrow(*cs).borrow_mut();

            match t_ref.as_mut().unwrap().wait() {
                Ok(()) => {
                    let tmp: I8F8 = tcn.temperature().unwrap().into();

                    let s: &mut dyn SerWrite<Error = serial::ErrorKind> = s_ref.as_mut().unwrap();
                    write!(s, "{}\n", I8F8!(1.8) * tmp + I8F8!(32)).unwrap();
                }
                _ => {}
            }
        })
    }
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
