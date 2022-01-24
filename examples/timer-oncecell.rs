//! Sharing data between a main thread and an interrupt handler safely.
//!
//! This example uses the externally-provided [once_cell][once] to safely share access to msp430
//! peripherals between a main thread and interrupt.
//!
//! The different between [OnceCell][once] and [RefCell][ref] is that setting the data contained
//! inside a [OnceCell][once] can be deferred to run time, but can only be set once. In contrast,
//! the data contained within a [RefCell][ref] can be set multiple times throughout a program, but
//! the contained data must be initialized at compile time. Additionally, [RefCell][ref] will
//! panic if a second thread tries to change its value while the first thread is mutating the
//! variable.
//!
//! The [Periperhals]({{device}}::Peripherals) type, and individual peripherals never need
//! to be modified. Therefore, [Periperhals]({{device}}::Peripherals) (or a subset of the
//! Periperhals _moved_ to another `struct`, if
//! [building](https://blog.japaric.io/brave-new-io/#freezing-the-clock-configuration)
//! higher-level abstractions) are good candidates to [`Send`](core::marker::Send) to a
//! [OnceCell][once]. [OnceCell][once] in general seems to have better space usage than
//! [RefCell][ref] due to its invariants.
//!
//! As with [timer] and [timer-unsafe], this example uses the `TIMER0_A1` interrupt to
//! blink LEDs on the [MSP-EXP430G2](http://www.ti.com/tool/MSP-EXP430G2) development kit.
//!
//! [once]: once_cell::unsync::OnceCell
//! [ref]: core::cell::RefCell
//!
//! ---

#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

extern crate panic_msp430;

use msp430::interrupt as mspint;
use msp430_rt::entry;
use {{device}}::{interrupt, Peripherals};
use once_cell::unsync::OnceCell;

static PERIPHERALS : mspint::Mutex<OnceCell<Peripherals>> = mspint::Mutex::new(OnceCell::new());

#[entry]
fn main(cs: CriticalSection) -> ! {
    let p = Peripherals::take().unwrap();

    let wdt = &p.WATCHDOG_TIMER;
    wdt.wdtctl
        .write(|w| w.wdtpw().password().wdthold().set_bit());

    let port_1_2 = &p.PORT_1_2;
    port_1_2
        .p1dir
        .modify(|_, w| w.p0().set_bit().p6().set_bit());
    port_1_2
        .p1out
        .modify(|_, w| w.p0().set_bit().p6().clear_bit());

    let clock = &p.SYSTEM_CLOCK;
    clock.bcsctl3.modify(|_, w| w.lfxt1s().lfxt1s_2());
    clock.bcsctl1.modify(|_, w| w.diva().diva_1());

    let timer = &p.TIMER0_A3;
    timer.taccr0.write(|w| w.bits(1200));
    timer.tactl.modify(|_, w| w.tassel().tassel_1().mc().mc_1());
    timer.tacctl1.modify(|_, w| w.ccie().set_bit());
    timer.taccr1.write(|w| w.bits(600));

    PERIPHERALS.borrow(cs).set(p).ok().unwrap();

    // Safe because interrupts are disabled after a reset.
    unsafe {
        mspint::enable();
    }

    loop {
        mspint::free(|_cs| {
            // Do something while interrupts are disabled.
        })
    }
}

#[interrupt]
fn TIMER0_A1(cs: CriticalSection) {
    let p = PERIPHERALS.borrow(cs).get().unwrap();

    let timer = &p.TIMER0_A3;
    timer.tacctl1.modify(|_, w| w.ccifg().clear_bit());

    let port_1_2 = &p.PORT_1_2;
    port_1_2
        .p1out
        .modify(|r, w| w.p0().bit(!r.p0().bit()).p6().bit(!r.p6().bit()));
}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
