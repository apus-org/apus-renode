#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

const DATA: u32 = 0xFF;
const N_CS: u32 = 1 << 8;
const D_C: u32 = 1 << 9;
const N_WR: u32 = 1 << 10;
const N_RD: u32 = 1 << 11;
const N_RESET: u32 = 1 << 10;

const PARAMETER: u16 = 0x100;
const RESET_LOW: u16 = 0x200;
const RESET_HIGH: u16 = 0x1000;
const DESELECT: u16 = 0x400;
const SELECT: u16 = 0x800;

/// Bus words, written by the test: bits 7–0 the byte, PARAMETER for D/C high, or one of
/// RESET_LOW, RESET_HIGH, DESELECT, SELECT
#[no_mangle]
static mut WORDS: [u16; 64] = [0; 64];
#[no_mangle]
static mut LEN: u32 = 0;

/// Pixels of FILL_COLOUR sent as parameters after the words, written by the test
#[no_mangle]
static mut FILL: u32 = 0;
#[no_mangle]
static mut FILL_COLOUR: u32 = 0;

/// Transfer number, written by the test last; DONE follows it when the words are sent
#[no_mangle]
static mut REQUEST: u32 = 0;
#[no_mangle]
static mut DONE: u32 = 0;

fn bsrr(port: &pac::gpiof::RegisterBlock, bits: u32) {
    port.bsrr.write(|w| unsafe { w.bits(bits) });
}

/// As the board's bus: one store puts the byte on DB0–DB7 and pulls nWR low, a second raises nWR
fn byte(gpioe: &pac::gpiof::RegisterBlock, byte: u8) {
    let byte = u32::from(byte);
    bsrr(gpioe, byte | (!byte & DATA) << 16 | N_WR << 16);
    bsrr(gpioe, N_WR);
}

fn send(gpioe: &pac::gpiof::RegisterBlock, gpiof: &pac::gpiof::RegisterBlock, word: u16) {
    match word {
        RESET_LOW => bsrr(gpiof, N_RESET << 16),
        RESET_HIGH => bsrr(gpiof, N_RESET),
        DESELECT => bsrr(gpioe, N_CS),
        SELECT => bsrr(gpioe, N_CS << 16),
        _ => {
            bsrr(gpioe, if word & PARAMETER != 0 { D_C } else { D_C << 16 });
            byte(gpioe, word as u8);
        }
    }
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC
        .ahbenr
        .modify(|_, w| w.iopeen().set_bit().iopfen().set_bit());

    let gpioe: &pac::gpiof::RegisterBlock = &p.GPIOE;
    let gpiof: &pac::gpiof::RegisterBlock = &p.GPIOF;
    bsrr(gpioe, N_RD | N_WR | N_CS << 16);
    bsrr(gpiof, N_RESET);
    // PE0–PE11 and PF10 outputs
    gpioe
        .moder
        .modify(|r, w| unsafe { w.bits(r.bits() & !0x00FF_FFFF | 0x0055_5555) });
    gpiof
        .moder
        .modify(|r, w| unsafe { w.bits(r.bits() & !(0b11 << 20) | 0b01 << 20) });

    loop {
        let request = unsafe { read_volatile(addr_of!(REQUEST)) };
        if request == unsafe { read_volatile(addr_of!(DONE)) } {
            continue;
        }

        let len = unsafe { read_volatile(addr_of!(LEN)) } as usize;
        let words = addr_of!(WORDS) as *const u16;
        for index in 0..len.min(64) {
            send(gpioe, gpiof, unsafe { read_volatile(words.add(index)) });
        }

        let colour = unsafe { read_volatile(addr_of!(FILL_COLOUR)) } as u16;
        bsrr(gpioe, D_C);
        for _ in 0..unsafe { read_volatile(addr_of!(FILL)) } {
            byte(gpioe, (colour >> 8) as u8);
            byte(gpioe, colour as u8);
        }

        unsafe { write_volatile(addr_of_mut!(DONE), request) };
    }
}
