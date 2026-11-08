#![no_std]

use stm32f0::stm32f0x2 as pac;

pub const SYSCLK_HZ: u32 = 48_000_000;

/// Sets the 48 MHz system clock from the 8 MHz crystal, AHB and APB undivided
pub fn clock(rcc: &pac::RCC, flash: &pac::FLASH) {
    rcc.cr.modify(|_, w| w.hseon().set_bit());
    while rcc.cr.read().hserdy().bit_is_clear() {}

    flash.acr.modify(|_, w| w.latency().ws1().prftbe().set_bit());
    rcc.cfgr
        .modify(|_, w| w.pllsrc().hse_div_prediv().pllmul().mul6());
    rcc.cr.modify(|_, w| w.pllon().set_bit());
    while rcc.cr.read().pllrdy().bit_is_clear() {}

    rcc.cfgr.modify(|_, w| w.sw().pll());
    while !rcc.cfgr.read().sws().is_pll() {}
}
