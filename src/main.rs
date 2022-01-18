#![no_main]
#![no_std]

extern crate panic_msp430; // For now, we only have an infinitely-looping panic handler.

use msp430::asm;
use msp430_rt::entry;

#[allow(unused)]
// Bring interrupt vectors into scope so the linker can see them; enabling the "rt"
// feature of {{device}} transitively enables the "device" feature of msp430-rt.
// This prevents default interrupt vectors from being generated.
use {{device}};

#[entry]
fn main() -> ! {
    asm::nop(); // If this isn't included, the empty infinite loop
                // gets optimized out during compiling. Probably safe
                // to remove in a "real" application.

    loop {
        // Application begins here.
    }
}

// The compiler will emit calls to the abort() compiler intrinsic if debug assertions are
// enabled (default for dev profile). MSP430 does not actually have meaningful abort() support
// so for now, we create our own in each application where debug assertions are present.
#[no_mangle]
extern "C" fn abort() -> ! {
    panic!();
}
