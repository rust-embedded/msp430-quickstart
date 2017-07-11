#![no_std]
#![feature(abi_msp430_interrupt)]

extern crate msp430;
#[macro_use(interrupt)]
extern crate msp430g2553;

use msp430::interrupt;
use msp430g2553::{PORT_1_2, TIMER0_A3, SYSTEM_CLOCK};

fn main() {
    interrupt::free(|cs| {
        // Disable watchdog
        let wdt = msp430g2553::WATCHDOG_TIMER.borrow(&cs);
        wdt.wdtctl.write(|w| {
            unsafe { w.bits(0x5A00) } // password
            .wdthold().set_bit()
        });

        let port_1_2 = PORT_1_2.borrow(cs);
        port_1_2.p1dir.modify(|_, w| w.p0().set_bit()
                                      .p6().set_bit());
        port_1_2.p1out.modify(|_, w| w.p0().set_bit()
                                      .p6().clear_bit());

        let clock = SYSTEM_CLOCK.borrow(cs);
        clock.bcsctl3.modify(|_, w| w.lfxt1s().lfxt1s_2());
        clock.bcsctl1.modify(|_, w| w.diva().diva_1());

        let timer = TIMER0_A3.borrow(cs);
        timer.ta0ccr0.write(|w| unsafe { w.bits(1200) });
        timer.ta0ctl.modify(|_, w| w.tassel().tassel_1()
                                    .mc().mc_1());
        timer.ta0cctl1.modify(|_, w| w.ccie().set_bit());
        timer.ta0ccr1.write(|w| unsafe { w.bits(600) });
    });

    unsafe {
        interrupt::enable();
    }

    loop {}
}

interrupt!(TIMER0_A1, timer_handler);
fn timer_handler() {
    interrupt::free(|cs| {
        let timer = TIMER0_A3.borrow(cs);
        timer.ta0cctl1.modify(|_, w| w.ccifg().clear_bit());

        let port_1_2 = PORT_1_2.borrow(cs);
        port_1_2.p1out.modify(|r, w| w.p0().bit(r.p6().bit())
                                      .p6().bit(r.p0().bit()));
    });
}
