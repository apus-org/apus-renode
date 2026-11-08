#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

const N_CS: u32 = 1 << 12;
const TIMEOUT: u32 = 10_000;

/// Bytes of one frame, written by the test
#[no_mangle]
static mut TX: [u8; 24] = [0; 24];
#[no_mangle]
static mut LEN: u32 = 0;

/// Bytes received in the frame, read by the test; 0xEE for a byte sent only
#[no_mangle]
static mut RX: [u8; 24] = [0; 24];

/// 1: first byte sent only, as stm32f0xx-hal's write, the rest full duplex as its read
#[no_mangle]
static mut HAL: u32 = 0;

/// 1: nCS stays high for the frame
#[no_mangle]
static mut DESELECTED: u32 = 0;

/// 1: nCS stays low after the frame
#[no_mangle]
static mut HOLD: u32 = 0;

/// Frame number, written by the test last; DONE follows it when the frame is over
#[no_mangle]
static mut REQUEST: u32 = 0;
#[no_mangle]
static mut DONE: u32 = 0;

/// 1 if the last frame timed out
#[no_mangle]
static mut FAILED: u32 = 0;

/// PENIRQ (PF2) level, read by the test
#[no_mangle]
static mut PINS: u32 = 0;

struct Timeout;

fn wait(done: impl Fn() -> bool) -> Result<(), Timeout> {
    for _ in 0..TIMEOUT {
        if done() {
            return Ok(());
        }
    }
    Err(Timeout)
}

fn send(spi: &pac::SPI1, byte: u8) {
    unsafe { write_volatile(addr_of!(spi.dr) as *mut u8, byte) };
}

fn receive(spi: &pac::SPI1) -> u8 {
    unsafe { read_volatile(addr_of!(spi.dr) as *const u8) }
}

fn exchange(spi: &pac::SPI1, byte: u8) -> Result<u8, Timeout> {
    send(spi, byte);
    wait(|| spi.sr.read().rxne().bit_is_set())?;
    Ok(receive(spi))
}

fn frame(spi: &pac::SPI1) -> Result<(), Timeout> {
    let len = (unsafe { read_volatile(addr_of!(LEN)) } as usize).min(24);
    let tx = addr_of!(TX) as *const u8;
    let rx = addr_of_mut!(RX) as *mut u8;
    let hal = unsafe { read_volatile(addr_of!(HAL)) } != 0;

    for index in 0..len {
        let byte = unsafe { read_volatile(tx.add(index)) };
        let received = if hal && index == 0 {
            spi.cr1.modify(|_, w| w.bidimode().set_bit().bidioe().set_bit());
            send(spi, byte);
            wait(|| {
                let sr = spi.sr.read();
                sr.txe().bit_is_set() && sr.bsy().bit_is_clear()
            })?;
            spi.cr1.modify(|_, w| w.bidimode().clear_bit().bidioe().clear_bit());
            0xEE
        } else {
            exchange(spi, byte)?
        };
        unsafe { write_volatile(rx.add(index), received) };
    }
    Ok(())
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC
        .ahbenr
        .modify(|_, w| w.iopeen().set_bit().iopfen().set_bit());
    p.RCC.apb2enr.modify(|_, w| w.spi1en().set_bit());

    let gpioe = &p.GPIOE;
    gpioe.bsrr.write(|w| unsafe { w.bits(N_CS) });
    gpioe
        .afrh
        .modify(|_, w| w.afrh13().af1().afrh14().af1().afrh15().af1());
    gpioe.moder.modify(|_, w| {
        w.moder12()
            .output()
            .moder13()
            .alternate()
            .moder14()
            .alternate()
            .moder15()
            .alternate()
    });
    p.GPIOF.pupdr.modify(|_, w| w.pupdr2().pull_up());

    let spi = &p.SPI1;
    // 8-bit frames, RXNE at one byte, mode 0, 750 kHz from 48 MHz
    spi.cr2
        .write(|w| unsafe { w.ds().bits(0b0111) }.frxth().set_bit());
    spi.cr1.write(|w| {
        w.mstr()
            .set_bit()
            .ssm()
            .set_bit()
            .ssi()
            .set_bit()
            .br()
            .bits(0b101)
            .spe()
            .set_bit()
    });

    loop {
        let pins = p.GPIOF.idr.read().idr2().bit() as u32;
        unsafe { write_volatile(addr_of_mut!(PINS), pins) };

        let request = unsafe { read_volatile(addr_of!(REQUEST)) };
        if request == unsafe { read_volatile(addr_of!(DONE)) } {
            continue;
        }

        let selected = unsafe { read_volatile(addr_of!(DESELECTED)) } == 0;
        if selected {
            gpioe.bsrr.write(|w| unsafe { w.bits(N_CS << 16) });
        }
        let failed = frame(spi).is_err() as u32;
        if unsafe { read_volatile(addr_of!(HOLD)) } == 0 {
            gpioe.bsrr.write(|w| unsafe { w.bits(N_CS) });
        }

        let pins = p.GPIOF.idr.read().idr2().bit() as u32;
        unsafe {
            write_volatile(addr_of_mut!(PINS), pins);
            write_volatile(addr_of_mut!(FAILED), failed);
            write_volatile(addr_of_mut!(DONE), request);
        }
    }
}
