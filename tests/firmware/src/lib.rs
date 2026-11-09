#![no_std]

use stm32f0::stm32f0x2 as pac;

pub const SYSCLK_HZ: u32 = 48_000_000;

/// Sets the 48 MHz system clock from the 8 MHz crystal, AHB and APB undivided
pub fn clock(rcc: &pac::RCC, flash: &pac::FLASH) {
    rcc.cr.modify(|_, w| w.hseon().set_bit());
    while rcc.cr.read().hserdy().bit_is_clear() {}

    flash
        .acr
        .modify(|_, w| w.latency().ws1().prftbe().set_bit());
    rcc.cfgr
        .modify(|_, w| w.pllsrc().hse_div_prediv().pllmul().mul6());
    rcc.cr.modify(|_, w| w.pllon().set_bit());
    while rcc.cr.read().pllrdy().bit_is_clear() {}

    rcc.cfgr.modify(|_, w| w.sw().pll());
    while !rcc.cfgr.read().sws().is_pll() {}
}

/// Polling steps before a transfer gives up: Renode never completes one to an unknown address
const TIMEOUT: u32 = 10_000;

/// I2C1 on PB6 (SCL) and PB7 (SDA), open drain on AF1, clocked by the 8 MHz HSI
pub struct I2c<'a>(&'a pac::I2C1);

#[derive(Debug)]
pub struct Timeout;

impl<'a> I2c<'a> {
    pub fn new(i2c: &'a pac::I2C1, rcc: &pac::RCC, gpiob: &pac::GPIOB) -> Self {
        rcc.ahbenr.modify(|_, w| w.iopben().set_bit());
        rcc.apb1enr.modify(|_, w| w.i2c1en().set_bit());
        gpiob
            .otyper
            .modify(|_, w| w.ot6().set_bit().ot7().set_bit());
        gpiob.afrl.modify(|_, w| w.afrl6().af1().afrl7().af1());
        gpiob
            .moder
            .modify(|_, w| w.moder6().alternate().moder7().alternate());
        // 100 kHz from 8 MHz
        i2c.timingr.write(|w| unsafe { w.bits(0x1042_0F13) });
        i2c.cr1.write(|w| w.pe().set_bit());
        Self(i2c)
    }

    fn wait(&self, done: impl Fn(&pac::i2c1::isr::R) -> bool) -> Result<(), Timeout> {
        for _ in 0..TIMEOUT {
            let isr = self.0.isr.read();
            if isr.nackf().bit_is_set() {
                self.0.icr.write(|w| w.nackcf().set_bit());
                return Err(Timeout);
            }
            if done(&isr) {
                return Ok(());
            }
        }
        Err(Timeout)
    }

    fn start(&self, address: u8, read: bool, len: usize, autoend: bool) {
        self.0.cr2.write(|w| {
            w.sadd()
                .bits(u16::from(address) << 1)
                .rd_wrn()
                .bit(read)
                .nbytes()
                .bits(len as u8)
                .autoend()
                .bit(autoend)
                .start()
                .set_bit()
        });
    }

    fn stop(&self) -> Result<(), Timeout> {
        self.wait(|isr| isr.stopf().bit_is_set())?;
        self.0.icr.write(|w| w.stopcf().set_bit());
        Ok(())
    }

    /// Sends `data`, ending with STOP if `stop`, else waiting for TC to start a read with
    pub fn write(&self, address: u8, data: &[u8], stop: bool) -> Result<(), Timeout> {
        self.start(address, false, data.len(), stop);
        for &byte in data {
            self.wait(|isr| isr.txis().bit_is_set())?;
            self.0.txdr.write(|w| w.txdata().bits(byte));
        }
        if stop {
            self.stop()
        } else {
            self.wait(|isr| isr.tc().bit_is_set())
        }
    }

    /// Receives `data`, ending with STOP
    pub fn read(&self, address: u8, data: &mut [u8]) -> Result<(), Timeout> {
        self.start(address, true, data.len(), true);
        for byte in data.iter_mut() {
            self.wait(|isr| isr.rxne().bit_is_set())?;
            *byte = self.0.rxdr.read().rxdata().bits();
        }
        self.stop()
    }
}
