#![no_std]
#![no_main]

use core::ptr::{addr_of_mut, read_volatile, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

/// MSTR, SPE, SSI, SSM
const MASTER: u32 = 0x0344;
/// FRXTH, 8-bit frames
const EIGHT_BIT: u32 = 0x1700;
const RXDMAEN: u32 = 1 << 0;
const TXDMAEN: u32 = 1 << 1;
/// EN, MINC; DIR for memory to peripheral
const RECEIVE: u32 = 0x0081;
const TRANSMIT: u32 = 0x0091;
const TIMEOUT: u32 = 10_000;

static SENT: [u8; 4] = [0x10, 0x11, 0x12, 0x13];

/// Bytes SPI1 and SPI2 received through DMA, read by the test
#[no_mangle]
static mut RX: [[u8; 4]; 2] = [[0; 4]; 2];

/// Bytes each receive channel still expected when the firmware stopped waiting
#[no_mangle]
static mut LEFT: [u32; 2] = [u32::MAX; 2];

fn transfer(spi: &pac::spi1::RegisterBlock, rx: &pac::dma1::CH, tx: &pac::dma1::CH, index: usize) {
    let dr = spi.dr.as_ptr() as u32;
    let into = unsafe { addr_of_mut!(RX).cast::<[u8; 4]>().add(index) };
    spi.cr2.write(|w| unsafe { w.bits(EIGHT_BIT | RXDMAEN) });
    spi.cr1.write(|w| unsafe { w.bits(MASTER) });
    rx.par.write(|w| unsafe { w.bits(dr) });
    rx.mar.write(|w| unsafe { w.bits(into as u32) });
    rx.ndtr.write(|w| unsafe { w.bits(4) });
    rx.cr.write(|w| unsafe { w.bits(RECEIVE) });
    tx.par.write(|w| unsafe { w.bits(dr) });
    tx.mar.write(|w| unsafe { w.bits(SENT.as_ptr() as u32) });
    tx.ndtr.write(|w| unsafe { w.bits(4) });
    tx.cr.write(|w| unsafe { w.bits(TRANSMIT) });
    spi.cr2.modify(|r, w| unsafe { w.bits(r.bits() | TXDMAEN) });
    for _ in 0..TIMEOUT {
        if rx.ndtr.read().bits() == 0 {
            break;
        }
    }
    unsafe {
        write_volatile(
            addr_of_mut!(LEFT).cast::<u32>().add(index),
            rx.ndtr.read().bits(),
        )
    };
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    p.RCC.ahbenr.modify(|_, w| w.dmaen().set_bit());
    p.RCC.apb2enr.modify(|_, w| w.spi1en().set_bit());
    p.RCC.apb1enr.modify(|_, w| w.spi2en().set_bit());
    transfer(&p.SPI1, &p.DMA1.ch2, &p.DMA1.ch3, 0);
    transfer(&p.SPI2, &p.DMA1.ch4, &p.DMA1.ch5, 1);
    let _ = unsafe { read_volatile(addr_of_mut!(RX)) };

    loop {
        cortex_m::asm::wfi();
    }
}
