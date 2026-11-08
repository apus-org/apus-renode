#![no_std]
#![no_main]

use core::ptr::{addr_of, addr_of_mut, read_volatile, write_volatile};

use cortex_m_rt::entry;
use panic_halt as _;
use stm32f0::stm32f0x2 as pac;

use apus_renode_firmware::{I2c, Timeout};

/// 7-bit address of the transfer, written by the test
#[no_mangle]
static mut ADDRESS: u32 = 0;

/// Bytes to send first, written by the test; none: read only
#[no_mangle]
static mut WRITE: [u8; 32] = [0; 32];
#[no_mangle]
static mut WRITE_LEN: u32 = 0;

/// Bytes received after the write, read by the test; none: write only
#[no_mangle]
static mut READ: [u8; 32] = [0; 32];
#[no_mangle]
static mut READ_LEN: u32 = 0;

/// 0: read after a repeated START, 1: write and read as two transfers, each ending with STOP
#[no_mangle]
static mut SEPARATE: u32 = 0;

/// Transfer number, written by the test last; DONE follows it when the transfer is over
#[no_mangle]
static mut REQUEST: u32 = 0;
#[no_mangle]
static mut DONE: u32 = 0;

/// 1 if the last transfer timed out, read by the test
#[no_mangle]
static mut FAILED: u32 = 0;

/// Line levels, read by the test: bit 0 RTC_MFP (PD3), bit 1 TEMP_ALERT (PD4)
#[no_mangle]
static mut PINS: u32 = 0;

fn transfer(i2c: &I2c) -> Result<(), Timeout> {
    let address = unsafe { read_volatile(addr_of!(ADDRESS)) } as u8;
    let write_len = unsafe { read_volatile(addr_of!(WRITE_LEN)) } as usize;
    let read_len = unsafe { read_volatile(addr_of!(READ_LEN)) } as usize;
    let separate = unsafe { read_volatile(addr_of!(SEPARATE)) } != 0;
    let write = unsafe { &(&*addr_of!(WRITE))[..write_len] };
    let read = unsafe { &mut (&mut *addr_of_mut!(READ))[..read_len] };

    if write_len > 0 {
        i2c.write(address, write, read_len == 0 || separate)?;
    }
    if read_len > 0 {
        i2c.read(address, read)?;
    }
    Ok(())
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    apus_renode_firmware::clock(&p.RCC, &p.FLASH);
    p.RCC.ahbenr.modify(|_, w| w.iopden().set_bit());
    let i2c = I2c::new(&p.I2C1, &p.RCC, &p.GPIOB);

    loop {
        let idr = p.GPIOD.idr.read();
        let pins = idr.idr3().bit() as u32 | (idr.idr4().bit() as u32) << 1;
        unsafe { write_volatile(addr_of_mut!(PINS), pins) };

        let request = unsafe { read_volatile(addr_of!(REQUEST)) };
        if request != unsafe { read_volatile(addr_of!(DONE)) } {
            let failed = transfer(&i2c).is_err() as u32;
            unsafe {
                write_volatile(addr_of_mut!(FAILED), failed);
                write_volatile(addr_of_mut!(DONE), request);
            }
        }
    }
}
