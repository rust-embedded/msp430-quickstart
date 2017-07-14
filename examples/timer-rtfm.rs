#![deny(warnings)]
#![feature(abi_msp430_interrupt)]
#![feature(proc_macro)]
#![no_std]

extern crate msp430;
extern crate msp430g2553;
#[macro_use(task)]
extern crate msp430_rtfm as rtfm;

use msp430::asm;
use rtfm::rtfm;

rtfm! {
    device: msp430g2553,

    init: {
        path: init,
    },

    idle: {
        path: idle,
    },

    tasks: {
        TIMER0_A1: {
            resources: [PORT_1_2, TIMER0_A3],
        },
    },
}

// This initialization function runs first and has full access to all the
// resources (peripherals and data)
// This function runs with interrupts disabled and *can't* be preempted
fn init(p: init::Peripherals) {
    // Disable watchdog
    p.WATCHDOG_TIMER.wdtctl.write(|w| unsafe {
        const PASSWORD: u16 = 0x5A00;
        w.bits(PASSWORD).wdthold().set_bit()
    });

    p.PORT_1_2
        .p1dir
        .modify(|_, w| w.p0().set_bit().p6().set_bit());
    p.PORT_1_2
        .p1out
        .modify(|_, w| w.p0().set_bit().p6().clear_bit());

    p.SYSTEM_CLOCK.bcsctl3.modify(|_, w| w.lfxt1s().lfxt1s_2());
    p.SYSTEM_CLOCK.bcsctl1.modify(|_, w| w.diva().diva_1());

    p.TIMER0_A3.ta0ccr0.write(|w| unsafe { w.bits(1200) });
    p.TIMER0_A3
        .ta0ctl
        .modify(|_, w| w.tassel().tassel_1().mc().mc_1());
    p.TIMER0_A3.ta0cctl1.modify(|_, w| w.ccie().set_bit());
    p.TIMER0_A3.ta0ccr1.write(|w| unsafe { w.bits(600) });
}

// The idle function runs right after `init`
// The interrupts are enabled at this point and `idle` and can be preempted
fn idle() -> ! {
    loop {
        // NOTE it seems this infinite loop gets optimized to `undef` if the NOP
        // is removed
        asm::nop()
    }
}

task!(TIMER0_A1, periodic);

// A task has access to the resources declared in the `rtfm!` macro
fn periodic(r: TIMER0_A1::Resources) {
    r.TIMER0_A3.ta0cctl1.modify(|_, w| w.ccifg().clear_bit());

    r.PORT_1_2
        .p1out
        .modify(|r, w| w.p0().bit(r.p6().bit()).p6().bit(r.p0().bit()));
}
