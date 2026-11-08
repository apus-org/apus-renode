//! Measures instructions per cycle: run on the board and in Renode, then compare the counts.
//! The board prints cycles, Renode prints instructions × 48 / PerformanceInMips.

#![no_std]
#![no_main]

use core::fmt::Write;
use core::hint::black_box;

use cortex_m::peripheral::{syst::SystClkSource, SYST};
use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

use apus_renode_firmware::SYSCLK_HZ;

const RELOAD: u32 = 0x00FF_FFFF;
const BAUD_RATE: u32 = 115_200;
const BUSY_CYCLES: u32 = 2_000_000;
const WORDS: usize = 512;
const PASSES: u32 = 200;
const BUS_WRITES: u32 = 100_000;
const WR: u32 = 1 << 10;

/// USART3 on PD8 (TX), the board's VCOM
struct Vcom<'a>(&'a pac::usart1::RegisterBlock);

impl Write for Vcom<'_> {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        for byte in text.bytes() {
            while self.0.isr.read().txe().bit_is_clear() {}
            self.0.tdr.write(|w| unsafe { w.bits(byte.into()) });
        }
        Ok(())
    }
}

fn ticks(syst: &mut SYST, work: impl FnOnce()) -> u32 {
    syst.clear_current();
    let start = SYST::get_current();
    work();
    let end = SYST::get_current();
    start.wrapping_sub(end) & RELOAD
}

fn arithmetic(buffer: &mut [u32; WORDS]) {
    let mut state = 0x1234_5678u32;
    for _ in 0..PASSES {
        for word in buffer.iter_mut() {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            *word = word.wrapping_add(state);
        }
    }
    black_box(buffer);
}

fn copy(source: &[u32; WORDS], target: &mut [u32; WORDS]) {
    for _ in 0..PASSES {
        target.copy_from_slice(black_box(source));
    }
    black_box(target);
}

/// Display data bus: byte on PE0–7 with WR low, then WR high, one BSRR store each
fn bus_writes(gpioe: &pac::gpioe::RegisterBlock) {
    for value in 0..BUS_WRITES {
        let byte = value & 0xFF;
        gpioe.bsrr.write(|w| unsafe { w.bits(byte | (!byte & 0xFF | WR) << 16) });
        gpioe.bsrr.write(|w| unsafe { w.bits(WR) });
    }
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);

    p.RCC.ahbenr.modify(|_, w| w.iopden().set_bit().iopeen().set_bit());
    p.RCC.apb1enr.modify(|_, w| w.usart3en().set_bit());
    // PD8 alternate function 0 = USART3_TX
    p.GPIOD.moder.modify(|_, w| w.moder8().alternate());
    p.USART3.brr.write(|w| unsafe { w.bits((SYSCLK_HZ + BAUD_RATE / 2) / BAUD_RATE) });
    p.USART3.cr1.write(|w| w.te().set_bit().ue().set_bit());
    // PE0–7 and PE10 (WR) as outputs
    p.GPIOE.moder.modify(|r, w| unsafe { w.bits(r.bits() & !0x0030_FFFF | 0x0010_5555) });

    let mut syst = cp.SYST;
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(RELOAD);
    syst.enable_counter();

    let mut source = [0u32; WORDS];
    let mut target = [0u32; WORDS];
    let mut vcom = Vcom(&p.USART3);

    for round in 0.. {
        let busy = ticks(&mut syst, || cortex_m::asm::delay(black_box(BUSY_CYCLES)));
        let arith = ticks(&mut syst, || arithmetic(&mut source));
        let copied = ticks(&mut syst, || copy(&source, &mut target));
        let bus = ticks(&mut syst, || bus_writes(&p.GPIOE));
        let _ = writeln!(vcom, "round {round} busy {busy} arith {arith} copy {copied} bus {bus}\r");
        cortex_m::asm::delay(SYSCLK_HZ / 2);
    }
    loop {}
}
