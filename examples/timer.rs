//! Sharing data between a main thread and an interrupt handler safely.
//!
//! This example uses the [libcore](core)-provided [RefCell](core::cell::RefCell) to safely share
//! access to msp430 peripherals between a main thread and interrupt.
//!
//! As with [timer-unsafe] and [timer-oncecell], this example uses the `TIMER0_A1` interrupt to
//! blink LEDs on the [MSP-EXP430G2](http://www.ti.com/tool/MSP-EXP430G2) development kit.
//!
//! ---

#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

extern crate panic_msp430;

use core::cell::RefCell;
use msp430::interrupt as mspint;
use msp430_rt::entry;
use msp430g2553::{interrupt, Peripherals};

static PERIPHERALS : mspint::Mutex<RefCell<Option<Peripherals>>> =
    mspint::Mutex::new(RefCell::new(None));

#[entry]
fn main(cs: CriticalSection) -> ! {
    let p = Peripherals::take().unwrap();

    let wdt = &p.WATCHDOG_TIMER;
    wdt.wdtctl.write(|w| {
        unsafe { w.bits(0x5A00) } // password
        .wdthold().set_bit()
    });

    let port_1_2 = &p.PORT_1_2;
    port_1_2.p1dir.modify(|_, w| w.p0().set_bit()
                                  .p6().set_bit());
    port_1_2.p1out.modify(|_, w| w.p0().set_bit()
                                  .p6().clear_bit());

    let clock = &p.SYSTEM_CLOCK;
    clock.bcsctl3.modify(|_, w| w.lfxt1s().lfxt1s_2());
    clock.bcsctl1.modify(|_, w| w.diva().diva_1());

    let timer = &p.TIMER0_A3;
    timer.ta0ccr0.write(|w| unsafe { w.bits(1200) });
    timer.ta0ctl.modify(|_, w| w.tassel().tassel_1()
                                .mc().mc_1());
    timer.ta0cctl1.modify(|_, w| w.ccie().set_bit());
    timer.ta0ccr1.write(|w| unsafe { w.bits(600) });

    *PERIPHERALS.borrow(&cs).borrow_mut() = Some(p);

    mspint::enable_cs(cs);

    loop {
        mspint::free(|_cs| {
            // Do something while interrupts are disabled.
        })
    }
}

#[interrupt]
fn TIMER0_A1(cs: CriticalSection) {
    let p_ref = PERIPHERALS.borrow(&cs).borrow();
    let p = p_ref.as_ref().unwrap();

    let timer = &p.TIMER0_A3;
    timer.ta0cctl1.modify(|_, w| w.ccifg().clear_bit());

    let port_1_2 = &p.PORT_1_2;
    port_1_2.p1out.modify(|r, w| w.p0().bit(!r.p0().bit())
                                  .p6().bit(!r.p6().bit()));
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
