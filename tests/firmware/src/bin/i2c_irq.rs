#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2::{self as pac, interrupt};

use apus_renode_firmware::I2c;

const TEMPERATURE: u8 = 0x48;

/// I2C1 handler entries, read by the test
#[no_mangle]
static mut ENTRIES: u32 = 0;

#[interrupt]
fn I2C1() {
    unsafe {
        write_volatile(
            addr_of_mut!(ENTRIES),
            read_volatile(addr_of!(ENTRIES)).wrapping_add(1),
        )
    };
    let i2c = unsafe { &*pac::I2C1::ptr() };
    i2c.cr1.modify(|_, w| w.tcie().clear_bit());
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    let i2c = I2c::new(&p.I2C1, &p.RCC, &p.GPIOB);
    p.I2C1.cr1.modify(|_, w| w.tcie().set_bit());
    unsafe { NVIC::unmask(pac::Interrupt::I2C1) };

    // pointer to TA without STOP: TC raises the interrupt
    i2c.write(TEMPERATURE, &[0], false).ok();

    loop {
        cortex_m::asm::wfi();
    }
}
