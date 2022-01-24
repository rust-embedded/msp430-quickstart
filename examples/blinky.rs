//! Basic "hello world" blink demo for the [MSP-EXP430G2](http://www.ti.com/tool/MSP-EXP430G2)
//! development kit using a software delay loop- in Rust!
//!
//! Although unnecessary for running the demo, this example also shows the syntax for declaring
//! an interrupt.
//!
//! ---

#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

extern crate panic_msp430;

use msp430::asm;
use msp430_rt::entry;
use {{device}}::interrupt;

fn delay(n: u16) {
    let mut i = 0;
    loop {
        asm::nop();

        i += 1;

        if i == n {
            break;
        }
    }
}

// P0 = red LED
// P6 = green LED
#[entry]
fn main() -> ! {
    let p = {{device}}::Peripherals::take().unwrap();

    // Disable watchdog
    let wd = p.WATCHDOG_TIMER;
    wd.wdtctl
        .write(|w| w.wdtpw().password().wdthold().set_bit());

    let p12 = p.PORT_1_2;

    // set P0 high and P6 low
    p12.p1out.modify(|_, w| w.p0().set_bit().p6().clear_bit());

    // Set P0 and P6 as outputs
    p12.p1dir.modify(|_, w| w.p0().set_bit().p6().set_bit());

    loop {
        delay(10_000);

        // toggle outputs
        p12.p1out
            .modify(|r, w| w.p0().bit(!r.p0().bit()).p6().bit(!r.p6().bit()));
    }
}

#[interrupt]
fn TIMER0_A0() {}

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
