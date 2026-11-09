#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

use apus_renode_firmware::SYSCLK_HZ;

const TICKS_PER_SECOND: u32 = 1_000;
const BAUD_RATE: u32 = 115_200;

/// SysTick interrupts since start, read by the test
#[no_mangle]
static mut MILLIS: u32 = 0;

#[exception]
fn SysTick() {
    unsafe {
        write_volatile(
            addr_of_mut!(MILLIS),
            read_volatile(addr_of!(MILLIS)).wrapping_add(1),
        )
    };
}

fn enable_transmitter(usart: &pac::usart1::RegisterBlock) {
    let divider = (SYSCLK_HZ + BAUD_RATE / 2) / BAUD_RATE;
    usart.brr.write(|w| unsafe { w.bits(divider) });
    usart.cr1.write(|w| w.te().set_bit().ue().set_bit());
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let mut cp = cortex_m::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);

    cp.SYST.set_clock_source(SystClkSource::Core);
    cp.SYST.set_reload(SYSCLK_HZ / TICKS_PER_SECOND - 1);
    cp.SYST.clear_current();
    cp.SYST.enable_interrupt();
    cp.SYST.enable_counter();

    p.RCC.apb2enr.modify(|_, w| w.usart1en().set_bit());
    p.RCC.apb1enr.modify(|_, w| {
        w.usart2en()
            .set_bit()
            .usart3en()
            .set_bit()
            .usart4en()
            .set_bit()
    });
    for usart in [&*p.USART1, &*p.USART2, &*p.USART3, &*p.USART4] {
        enable_transmitter(usart);
    }

    loop {
        cortex_m::asm::wfi();
    }
}
