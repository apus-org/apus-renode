#![no_std]
#![no_main]

use core::ptr::{addr_of, read_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

/// GPIOC pins of the active-low outputs, in the order the tests select them: LED3, LED4, LED5
const OUTPUT_PINS: [u32; 3] = [0, 1, 2];
const NONE: u32 = u32::MAX;

/// Index into OUTPUT_PINS of the one output to switch on, written by the test
#[no_mangle]
static mut OUTPUT: u32 = NONE;

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC.ahbenr.modify(|_, w| w.iopcen().set_bit());

    let outputs = OUTPUT_PINS.iter().fold(0, |mask, pin| mask | 1 << pin);
    // high is off: set before the pins become outputs
    p.GPIOC.bsrr.write(|w| unsafe { w.bits(outputs) });
    p.GPIOC.moder.modify(|r, w| {
        let moder = OUTPUT_PINS
            .iter()
            .fold(r.bits(), |moder, pin| moder & !(0b11 << (2 * pin)) | 0b01 << (2 * pin));
        unsafe { w.bits(moder) }
    });

    loop {
        let selected = unsafe { read_volatile(addr_of!(OUTPUT)) };
        let on = OUTPUT_PINS.get(selected as usize).map_or(0, |pin| 1 << pin);
        p.GPIOC.bsrr.write(|w| unsafe { w.bits(outputs & !on | on << 16) });
    }
}
