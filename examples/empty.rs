#![no_main]
#![no_std]

extern crate panic_msp430;

use msp430_rt::entry;

#[entry]
fn main() -> ! {
    loop { }
}
