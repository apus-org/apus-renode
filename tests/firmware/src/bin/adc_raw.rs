#![no_std]
#![no_main]

use core::ptr::{addr_of_mut, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

/// ADC_IN14 on PC4, the potentiometer
const POT_CHANNEL: u32 = 14;
/// internal reference
const VREFINT_CHANNEL: u32 = 17;
/// PCLK / 4 = 12 MHz, the ADC allows at most 14 MHz
const ADC_CLOCK_DIVIDER: u32 = 4;
/// ADC clocks between the end of calibration and ADEN
const AFTER_CALIBRATION_ADC_CLOCKS: u32 = 4;

/// Raw 12-bit conversions, read by the test
#[no_mangle]
static mut POT: u32 = 0;
#[no_mangle]
static mut VREFINT: u32 = 0;

fn convert(adc: &pac::ADC, channel: u32) -> u32 {
    adc.chselr.write(|w| unsafe { w.bits(1 << channel) });
    adc.cr.modify(|_, w| w.adstart().set_bit());
    while adc.isr.read().eoc().bit_is_clear() {}
    adc.dr.read().bits()
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
    p.RCC.apb2enr.modify(|_, w| w.adcen().set_bit());
    p.GPIOC.moder.modify(|_, w| w.moder4().analog());

    let adc = &p.ADC;
    adc.cfgr2.write(|w| w.ckmode().pclk_div4());
    adc.cr.modify(|_, w| w.adcal().set_bit());
    while adc.cr.read().adcal().bit_is_set() {}
    cortex_m::asm::delay(ADC_CLOCK_DIVIDER * AFTER_CALIBRATION_ADC_CLOCKS);
    adc.ccr.modify(|_, w| w.vrefen().set_bit());
    // the internal reference needs the longest sampling time
    adc.smpr.write(|w| w.smp().cycles239_5());
    adc.cr.modify(|_, w| w.aden().set_bit());
    while adc.isr.read().adrdy().bit_is_clear() {}

    loop {
        let pot = convert(adc, POT_CHANNEL);
        let vrefint = convert(adc, VREFINT_CHANNEL);
        unsafe {
            write_volatile(addr_of_mut!(POT), pot);
            write_volatile(addr_of_mut!(VREFINT), vrefint);
        }
    }
}
