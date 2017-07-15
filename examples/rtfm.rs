#![deny(warnings)]
#![feature(abi_msp430_interrupt)]
#![feature(const_fn)]
#![feature(proc_macro)]
#![no_std]

extern crate msp430;
extern crate msp430g2553;
#[macro_use(task)]
extern crate msp430_rtfm as rtfm;

use rtfm::app;

app! {
    device: msp430g2553,

    resources: {
        SHARED: u32 = 0;
    },

    idle: {
        resources: [SHARED],
    },

    tasks: {
        TIMER0_A1: {
            resources: [SHARED],
        },
    },
}

fn init(_p: init::Peripherals, _r: init::Resources) {}

fn idle(mut r: idle::Resources) -> ! {
    loop {
        // ..

        // to access a *shared* resource from `idle` a critical section is
        // needed
        rtfm::atomic(|cs| { **r.SHARED.borrow_mut(cs) += 1; });

        // ..
    }
}

task!(TIMER0_A1, periodic);

fn periodic(r: TIMER0_A1::Resources) {
    // interrupts don't need a critical section to access resources
    **r.SHARED += 1;
}
