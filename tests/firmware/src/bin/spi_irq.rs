#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2::{self as pac, interrupt};

/// MSTR, SPE, SSI, SSM
const MASTER: u32 = 0x0344;
/// FRXTH, 8-bit frames, RXNEIE
const RECEIVE_INTERRUPT: u32 = 0x1740;

/// SPI1 and SPI2 handler entries, read by the test
#[no_mangle]
static mut ENTRIES: [u32; 2] = [0; 2];

fn count(spi: &pac::spi1::RegisterBlock, index: usize) {
    unsafe {
        read_volatile(spi.dr.as_ptr().cast::<u8>());
        let entries = addr_of_mut!(ENTRIES).cast::<u32>().add(index);
        write_volatile(entries, read_volatile(entries).wrapping_add(1));
    }
}

#[interrupt]
fn SPI1() {
    count(unsafe { &*pac::SPI1::ptr() }, 0);
}

#[interrupt]
fn SPI2() {
    count(unsafe { &*pac::SPI2::ptr() }, 1);
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    p.RCC.apb2enr.modify(|_, w| w.spi1en().set_bit());
    p.RCC.apb1enr.modify(|_, w| w.spi2en().set_bit());
    unsafe {
        NVIC::unmask(pac::Interrupt::SPI1);
        NVIC::unmask(pac::Interrupt::SPI2);
    }
    for spi in [&*p.SPI1, &*p.SPI2] {
        spi.cr2.write(|w| unsafe { w.bits(RECEIVE_INTERRUPT) });
        spi.cr1.write(|w| unsafe { w.bits(MASTER) });
        unsafe { write_volatile(spi.dr.as_ptr().cast::<u8>(), 0x5A) };
    }
    let _ = unsafe { read_volatile(addr_of!(ENTRIES)) };

    // a WFI right after the writes would hang Renode: the interrupt is taken but the CPU stays asleep
    loop {
        cortex_m::asm::nop();
    }
}
