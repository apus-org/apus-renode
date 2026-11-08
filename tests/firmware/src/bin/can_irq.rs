#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2::{self as pac, interrupt};

/// standard identifiers, routed to FIFO 0 and FIFO 1 by filters 0 and 1
const FIFO_IDS: [u32; 2] = [0x100, 0x200];
const STID_SHIFT: u32 = 21;

const NONE: u32 = u32::MAX;
const SLKIE: u32 = 17;

/// IER bit of the one interrupt source to enable (TMEIE 0, FMPIE0 1, FMPIE1 4, SLKIE 17), written by the test
#[no_mangle]
static mut SOURCE: u32 = NONE;

/// CEC_CAN handler entries, read by the test
#[no_mangle]
static mut ENTRIES: u32 = 0;

#[interrupt]
fn CEC_CAN() {
    unsafe {
        write_volatile(
            addr_of_mut!(ENTRIES),
            read_volatile(addr_of!(ENTRIES)).wrapping_add(1),
        )
    };
    let can = unsafe { &*pac::CAN::ptr() };
    can.tsr.write(|w| w.rqcp0().set_bit());
    can.msr.write(|w| w.slaki().set_bit());
    for fifo in &can.rfr {
        if fifo.read().fmp().bits() > 0 {
            fifo.write(|w| w.rfom().set_bit());
        }
    }
}

fn transmit(can: &pac::CAN, id: u32) {
    while can.tsr.read().tme0().bit_is_clear() {}
    let mailbox = &can.tx[0];
    mailbox.tdtr.write(|w| unsafe { w.dlc().bits(1) });
    mailbox.tdlr.write(|w| unsafe { w.bits(id) });
    mailbox
        .tir
        .write(|w| unsafe { w.bits(id << STID_SHIFT) }.txrq().set_bit());
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC.apb1enr.modify(|_, w| w.canen().set_bit());

    let can = &p.CAN;
    can.mcr
        .modify(|_, w| w.sleep().clear_bit().inrq().set_bit());
    while can.msr.read().inak().bit_is_clear() {}
    // 500 kbit/s at 48 MHz: 6 × 16 time quanta, looped back without the bus
    can.btr.write(|w| unsafe {
        w.lbkm()
            .set_bit()
            .silm()
            .set_bit()
            .brp()
            .bits(5)
            .ts1()
            .bits(12)
            .ts2()
            .bits(1)
    });

    can.fmr.modify(|_, w| w.finit().set_bit());
    for (bank, id) in FIFO_IDS.iter().enumerate() {
        can.fb[bank]
            .fr1
            .write(|w| unsafe { w.bits(id << STID_SHIFT) });
        can.fb[bank]
            .fr2
            .write(|w| unsafe { w.bits(id << STID_SHIFT) });
    }
    // banks 0 and 1: 32-bit identifier lists, bank 1 into FIFO 1
    can.fm1r.write(|w| unsafe { w.bits(0b11) });
    can.fs1r.write(|w| unsafe { w.bits(0b11) });
    can.ffa1r.write(|w| unsafe { w.bits(0b10) });
    can.fa1r.write(|w| unsafe { w.bits(0b11) });
    can.fmr.modify(|_, w| w.finit().clear_bit());

    can.mcr.modify(|_, w| w.inrq().clear_bit());
    while can.msr.read().inak().bit_is_set() {}

    let source = loop {
        let source = unsafe { read_volatile(addr_of!(SOURCE)) };
        if source != NONE {
            break source;
        }
    };
    can.ier.write(|w| unsafe { w.bits(1 << source) });
    unsafe { NVIC::unmask(pac::Interrupt::CEC_CAN) };

    if source == SLKIE {
        can.mcr.modify(|_, w| w.sleep().set_bit());
    } else {
        for id in FIFO_IDS {
            transmit(can, id);
        }
    }

    loop {
        cortex_m::asm::wfi();
    }
}
