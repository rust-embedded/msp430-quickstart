#![no_main]
#![no_std]

extern crate panic_msp430; // For now, we only have an infinitely-loop panic handler.

use msp430::asm;
use msp430_rt::entry;

#[allow(unused)]
// Bring interrupt vectors into scope so the linker can see them; enabling the "rt"
// feature of msp430g2553 transitively enables the "device" feature of msp430-rt.
// This prevents default interrupt vectors from being generated.
use msp430g2553;

#[entry]
fn main() -> ! {
    asm::nop(); // If this isn't included, the empty infinite loop
                // gets optimized out during compiling. Probably safe
                // to remove in a "real" application.

    loop {
        // Application begins here.
    }
}
