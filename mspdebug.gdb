target remote localhost:2000

# print demangled symbols
set print asm-demangle on

# set backtrace limit to not have infinite backtrace loops
set backtrace limit 32

# List $pc- Print assembly instructions of surrounding $pc context.
define lpc
    x/10i ($pc - 10)
end

# When the processor stops, show the assembly corresponding to the
# next high-level source line.
# set disassemble-next-line on

# On a given breakpoint, you may wish to run the "disassemble" gdb command to
# get context of your surroundings.
# command <bpnum>
# silent
# disas
# cont
# <newline>

# mspdebug will not erase before loading unless you tell it to.
monitor erase
load

# Force msp430 to reread the reset vector and get back to the entry point.
# If this line is omitted, it's pretty easy to get errors like:
# * fet: FET returned error code 16 (Could not single step device)
# * fet: FET returned error code 17 (Could not run device (to breakpoint))
monitor reset

# start the process but immediately halt the processor
stepi
