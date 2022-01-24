use core::convert::Infallible;

use embedded_hal::i2c::{
    self,
    blocking::{Read as I2cRead, Write as I2cWrite}
};
use embedded_hal::serial::{self, nb::Write as SerWrite};
use embedded_hal::timer::{nb::CountDown, Periodic};
use embedded_hal::watchdog::blocking::{Disable, Enable, Watchdog};
use nb::Error as NbError;
use nb::Result as NbResult;

pub struct Timer {
    inner: {{device}}::TIMER0_A3,
    elapsed: bool,
}

// Interrupt-driven non-blocking timer.
impl Timer {
    pub fn new(inner: {{device}}::TIMER0_A3) -> Self {
        // 6kHz timer using AUX clk.
        inner.tactl.modify(|_, w| w.tassel().tassel_1().mc().mc_1());
        inner.tacctl1.modify(|_, w| w.ccie().set_bit());

        Timer {
            inner,
            elapsed: false,
        }
    }

    pub fn timer_int(&mut self) {
        self.elapsed = true;
        self.inner.tacctl1.modify(|_, w| w.ccifg().clear_bit());
    }
}

impl CountDown for Timer {
    type Error = Infallible;
    type Time = u16;

    fn start<T>(&mut self, count: T) -> Result<(), Self::Error>
    where
        T: Into<Self::Time>,
    {
        self.inner.taccr0.write(|w| w.bits(count.into()));
        Ok(())
    }

    fn wait(&mut self) -> NbResult<(), Self::Error> {
        if self.elapsed {
            self.elapsed = false;
            return Ok(());
        }

        return Err(NbError::WouldBlock);
    }
}

impl Periodic for Timer {}

pub struct Serial {
    inner: {{device}}::USCI_A0_UART_MODE,
}

impl Serial {
    pub fn new(inner: {{device}}::USCI_A0_UART_MODE) -> Self {
        inner.uca0ctl1.modify(|_, w| w.ucswrst().set_bit());
        inner.uca0ctl1.modify(|_, w| w.ucssel().ucssel_2()); // Submain clock for UART (1.1 MHz)
        inner.uca0ctl0.modify(|_, w| w.ucsync().clear_bit()); // UART mode
        inner.uca0br0.write(|w| w.bits(110)); // INT(1.1MHz/9600) = 114, but this worked better for me.
        inner.uca0br1.write(|w| w.bits(0));
        inner.uca0mctl.modify(|_, w| w.ucbrs().bits(0)); // ROUND(8*(1.1MHz/9600 - INT(1.1MHz/9600))) = 5,
                                                         // but this worked better for me.
        inner.uca0ctl1.modify(|_, w| w.ucswrst().clear_bit());

        Serial { inner }
    }
}

impl SerWrite<u8> for Serial {
    type Error = serial::ErrorKind;

    fn write(&mut self, word: u8) -> NbResult<(), Self::Error> {
        if self.inner.uca0stat.read().ucbusy().bit_is_set() {
            Err(NbError::WouldBlock)
        } else {
            self.inner.uca0txbuf.write(|w| w.bits(word));
            Ok(())
        }
    }

    fn flush(&mut self) -> NbResult<(), Self::Error> {
        Ok(())
    }
}

pub struct I2c {
    inner: {{device}}::USCI_B0_I2C_MODE,
    ifg: Ucb0Ifg,
}

impl I2c {
    pub fn new(inner: {{device}}::USCI_B0_I2C_MODE, ifg: Ucb0Ifg) -> Self {
        inner.ucb0ctl1.modify(|_, w| w.ucswrst().set_bit());
        inner.ucb0ctl1.modify(|_, w| w.ucssel().ucssel_2()); // Submain clock for I2C (1.1 MHz)
        inner
            .ucb0ctl0
            .modify(|_, w| w.ucsync().set_bit().ucmode().ucmode_3().ucmst().set_bit()); // I2C mode

        inner.ucb0br0.write(|w| w.bits(11)); // INT(1.1MHz/11) = 100kHz
        inner.ucb0br1.write(|w| w.bits(0));

        inner.ucb0ctl1.modify(|_, w| w.ucswrst().clear_bit());

        I2c { inner, ifg }
    }
}

impl I2cRead for I2c {
    // FIXME: Handle various error cases.
    type Error = i2c::ErrorKind;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.inner
            .ucb0i2csa
            .write(|w| w.ucsa().bits(address.into()));
        self.inner.ucb0ctl1.modify(|_, w| w.uctr().clear_bit());
        self.inner.ucb0ctl1.modify(|_, w| w.uctxstt().set_bit()); // Generate start condition.

        // Wait until peripheral responds.
        // FIXME: Check for UCNACKIFG bit, error xfer (and send stop?) if so.
        while self.inner.ucb0ctl1.read().uctxstt().bit_is_set() { }

        if let Some((last, all_but_last)) = buffer.split_last_mut() {
            for b in all_but_last {
                while self.ifg.ucb0rxifg.bit_is_clear() { }
                *b = self.inner.ucb0rxbuf.read().ucb0rxbuf().bits();
            }

            // Send stop bit immediately by triggering the stop before reading buffer.
            // If single byte to be received, we have to set stop bit WHILE the byte is being
            // received. This handles both.
            self.inner.ucb0ctl1.modify(|_, w| w.uctxstp().set_bit());
            while self.ifg.ucb0rxifg.bit_is_clear() { }
            *last = self.inner.ucb0rxbuf.read().ucb0rxbuf().bits();
        }

        Ok(())
    }
}

impl I2cWrite for I2c {
    // FIXME: Handle various error cases.
    type Error = i2c::ErrorKind;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.ucb0i2csa.write(|w| w.ucsa().bits(addr.into()));
        self.inner.ucb0ctl1.modify(|_, w| w.uctr().set_bit()); // Transmitter mode.
        self.inner.ucb0ctl1.modify(|_, w| w.uctxstt().set_bit()); // Generate start condition.

        while self.ifg.ucb0txifg.bit_is_clear() { } // Wait until ready. Required?

        // FIXME: Check for UCNACKIFG/UCALIFG bit, error xfer (and send stop?) if so.
        for b in bytes {
            self.inner.ucb0txbuf.write(|w| w.bits(*b));
            while self.ifg.ucb0txifg.bit_is_clear() { }
        }

        // Regardless of single byte or multi-byte xfer, setting STP bit immediately after
        // the data starts to be sent should work.
        self.inner.ucb0ctl1.modify(|_, w| w.uctxstp().set_bit());

        Ok(())
    }
}

pub struct WatchdogTimer {
    inner: {{device}}::WATCHDOG_TIMER,
}

impl WatchdogTimer {
    pub fn new(inner: {{device}}::WATCHDOG_TIMER) -> Self {
        WatchdogTimer { inner }
    }
}

impl Enable for WatchdogTimer {
    type Error = Infallible;
    type Time = WatchdogDivider;
    type Target = WatchdogTimer;

    fn start<T>(self, period: T) -> Result<Self::Target, Self::Error>
    where
        T: Into<Self::Time>,
    {
        self.inner.wdtctl.write(|w| {
            w.wdtpw()
                .password()
                .wdthold()
                .clear_bit()
                .wdtis()
                .bits(period.into() as u8)
        });

        Ok(self)
    }
}

impl Disable for WatchdogTimer {
    type Error = Infallible;
    type Target = WatchdogTimer;

    fn disable(self) -> Result<Self::Target, Self::Error> {
        self.inner
            .wdtctl
            .write(|w| w.wdtpw().password().wdthold().set_bit());
        Ok(self)
    }
}

impl Watchdog for WatchdogTimer {
    type Error = Infallible;

    fn feed(&mut self) -> Result<(), Self::Error> {
        self.inner
            .wdtctl
            .write(|w| w.wdtpw().password().wdtcntcl().set_bit());
        Ok(())
    }
}

#[repr(u8)]
#[allow(unused)]
pub enum WatchdogDivider {
    By32768 = 0,
    By8192 = 1,
    By512 = 2,
    By64 = 3,
}

// Struct that allows fine grained splitting of SFRs that are shared between peripherals, so that
// HAL impls can only access the registers they need. Functionality implemented on as-needed basis.
pub struct SfrIfg {
    pub ucb0ifg: Ucb0Ifg,
}

impl SfrIfg {
    pub fn new(_token: {{device}}::SPECIAL_FUNCTION) -> Self {

        // SAFETY: Thanks to the input arg, we have already either:
        // 1. Safely acquired the peripherals at this point, and thus another thread can't acquire
        // them without a Mutex+CriticalSection, or some other synchronization mechanism. Or...
        // 2. We already have opted into unsafety, at which point anything goes.
        let ucb0txifg =
            unsafe { Ucb0TxIfg::new({{device}}::Peripherals::steal().SPECIAL_FUNCTION) };
        let ucb0rxifg =
            unsafe { Ucb0RxIfg::new({{device}}::Peripherals::steal().SPECIAL_FUNCTION) };

        SfrIfg {
            ucb0ifg: Ucb0Ifg {
                ucb0txifg,
                ucb0rxifg,
            },
        }
    }
}

pub struct Ucb0Ifg {
    ucb0txifg: Ucb0TxIfg,
    ucb0rxifg: Ucb0RxIfg,
}

pub struct Ucb0TxIfg {
    inner: {{device}}::SPECIAL_FUNCTION,
}

impl Ucb0TxIfg {
    fn new(inner: {{device}}::SPECIAL_FUNCTION) -> Self {
        Ucb0TxIfg { inner }
    }

    #[allow(unused)]
    fn bit_is_set(&self) -> bool {
        self.inner.ifg2.read().ucb0txifg().bit_is_set()
    }

    fn bit_is_clear(&self) -> bool {
        self.inner.ifg2.read().ucb0txifg().bit_is_clear()
    }
}

pub struct Ucb0RxIfg {
    inner: {{device}}::SPECIAL_FUNCTION,
}

impl Ucb0RxIfg {
    fn new(inner: {{device}}::SPECIAL_FUNCTION) -> Self {
        Ucb0RxIfg { inner }
    }

    #[allow(unused)]
    fn bit_is_set(&self) -> bool {
        self.inner.ifg2.read().ucb0rxifg().bit_is_set()
    }

    fn bit_is_clear(&self) -> bool {
        self.inner.ifg2.read().ucb0rxifg().bit_is_clear()
    }
}
