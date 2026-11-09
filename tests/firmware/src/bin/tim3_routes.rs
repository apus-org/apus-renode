#![no_std]
#![no_main]

use core::ptr::{addr_of, read_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

use apus_renode_firmware::SYSCLK_HZ;

const PWM_HZ: u32 = 1_000;
const PERIOD: u32 = SYSCLK_HZ / PWM_HZ;
const CHANNELS: u32 = 4;
const NONE: u32 = u32::MAX;

/// TIM3 channel (0 = CH1 … 3 = CH4) whose output is enabled, written by the test
#[no_mangle]
static mut CHANNEL: u32 = NONE;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());
    p.RCC.apb1enr.modify(|_, w| w.tim3en().set_bit());

    // PC6–PC9: TIM3 CH1–CH4 on AF0
    p.GPIOC.afrl.modify(|_, w| w.afrl6().af0().afrl7().af0());
    p.GPIOC.afrh.modify(|_, w| w.afrh8().af0().afrh9().af0());
    p.GPIOC.moder.modify(|_, w| {
        w.moder6()
            .alternate()
            .moder7()
            .alternate()
            .moder8()
            .alternate()
            .moder9()
            .alternate()
    });

    let tim3 = &p.TIM3;
    tim3.psc.write(|w| unsafe { w.bits(0) });
    tim3.arr.write(|w| unsafe { w.bits(PERIOD - 1) });
    for ccr in tim3.ccr.iter() {
        ccr.write(|w| unsafe { w.bits(PERIOD / 2) });
    }
    tim3.ccmr1_output()
        .write(|w| w.oc1m().pwm_mode1().oc2m().pwm_mode1());
    tim3.ccmr2_output()
        .write(|w| w.oc3m().pwm_mode1().oc4m().pwm_mode1());
    tim3.egr.write(|w| w.ug().set_bit());
    tim3.cr1.write(|w| w.cen().set_bit());

    loop {
        let selected = unsafe { read_volatile(addr_of!(CHANNEL)) };
        // CCxE is bit 4 × channel
        let enable = if selected < CHANNELS {
            1 << (4 * selected)
        } else {
            0
        };
        tim3.ccer.write(|w| unsafe { w.bits(enable) });
    }
}
