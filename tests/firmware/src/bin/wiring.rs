#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

/// GPIOC pins of the active-low outputs, in the order the tests select them: LED3, LED4, LED5, red, green, blue, buzzer
const OUTPUT_PINS: [u32; 7] = [0, 1, 2, 6, 7, 8, 9];
const NONE: u32 = u32::MAX;

/// Index into OUTPUT_PINS of the one output to switch on, written by the test
#[no_mangle]
static mut OUTPUT: u32 = NONE;

/// Button inputs, read by the test: bit 0 WAKE_UP (PA0), bit 1 USER0 (PD0), bit 2 USER1 (PD1)
#[no_mangle]
static mut BUTTONS: u32 = 0;

/// Sets `pins` of a port to inputs with pull-down, as the buttons drive high when pressed
macro_rules! pull_down_inputs {
    ($port:expr, $pins:expr) => {{
        let pins: &[u32] = $pins;
        $port.moder.modify(|r, w| {
            let moder = pins
                .iter()
                .fold(r.bits(), |moder, pin| moder & !(0b11 << (2 * pin)));
            unsafe { w.bits(moder) }
        });
        $port.pupdr.modify(|r, w| {
            let pupdr = pins.iter().fold(r.bits(), |pupdr, pin| {
                pupdr & !(0b11 << (2 * pin)) | 0b10 << (2 * pin)
            });
            unsafe { w.bits(pupdr) }
        });
    }};
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC
        .ahbenr
        .modify(|_, w| w.iopaen().set_bit().iopcen().set_bit().iopden().set_bit());

    let outputs = OUTPUT_PINS.iter().fold(0, |mask, pin| mask | 1 << pin);
    // high is off: set before the pins become outputs
    p.GPIOC.bsrr.write(|w| unsafe { w.bits(outputs) });
    p.GPIOC.moder.modify(|r, w| {
        let moder = OUTPUT_PINS.iter().fold(r.bits(), |moder, pin| {
            moder & !(0b11 << (2 * pin)) | 0b01 << (2 * pin)
        });
        unsafe { w.bits(moder) }
    });

    pull_down_inputs!(p.GPIOA, &[0]);
    pull_down_inputs!(p.GPIOD, &[0, 1]);

    loop {
        let wake_up = p.GPIOA.idr.read().idr0().bit() as u32;
        let user = p.GPIOD.idr.read().bits() & 0b11;
        unsafe { write_volatile(addr_of_mut!(BUTTONS), wake_up | user << 1) };

        let selected = unsafe { read_volatile(addr_of!(OUTPUT)) };
        let on = OUTPUT_PINS.get(selected as usize).map_or(0, |pin| 1 << pin);
        p.GPIOC
            .bsrr
            .write(|w| unsafe { w.bits(outputs & !on | on << 16) });
    }
}
