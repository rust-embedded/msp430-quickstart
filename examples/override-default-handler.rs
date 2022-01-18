//! Overriding the default interrupt handler with user code.
//!
//! The default handler is used for _any_ interrupts whose handler is not explicitly defined
//! in the application. Normally an infinite loop, `DefaultHandler` can be overridden based on
//! a user's needs.
//!
//! This demo is meant to more to be compiled and then examined with `msp430-elf-objdump` and
//! other [binutils](https://www.gnu.org/software/binutils/) programs, rather than run on
//! a development board.
//!
//! ---

#![no_main]
#![no_std]
#![feature(abi_msp430_interrupt)]

extern crate panic_msp430;

use msp430_rt::entry;
use {{device}}::interrupt;

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

#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
