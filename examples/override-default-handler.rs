#![no_std]

#[macro_use(default_handler)]
extern crate msp430g2553;

use core::ptr;

fn main() {}

default_handler!(handler);

static mut X: u16 = 0;

fn handler() {
    unsafe {
        ptr::write_volatile(&mut X, ptr::read_volatile(&X) + 1);
    }
}
