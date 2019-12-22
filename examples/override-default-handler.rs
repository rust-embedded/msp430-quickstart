#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

extern crate panic_msp430;

use msp430_rt::entry;
use msp430g2553::interrupt;

use core::ptr;

#[entry]
fn main() -> ! {
    loop { }
}

static mut X: u16 = 0;

#[interrupt]
fn DefaultHandler() {
    unsafe {
        ptr::write_volatile(&mut X, ptr::read_volatile(&X) + 1);
    }
}
