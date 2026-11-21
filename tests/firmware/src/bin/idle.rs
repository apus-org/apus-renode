#![no_std]
#![no_main]

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0 as _;

#[entry]
fn main() -> ! {
    loop {
        cortex_m::asm::wfi();
    }
}
