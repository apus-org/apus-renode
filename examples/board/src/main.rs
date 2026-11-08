#![no_std]
#![no_main]

use core::fmt::Write;

use cortex_m_rt::entry;
use defmt_rtt as _;
use embedded_graphics::{
    mono_font::{
        ascii::{FONT_10X20, FONT_6X10},
        MonoTextStyle, MonoTextStyleBuilder,
    },
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use heapless::String;
use panic_probe as _;

use apus::board::Board;
use apus::bsp::{
    display::{DISPLAY_HEIGHT, DISPLAY_WIDTH},
    potentiometer::MAX_VALUE,
    rtc::{DateTimeAccess, NaiveDate},
    touch::TouchCalibrationData,
};
use apus::hal::pac;
use apus::prelude::*;

/// Raw touch readings at the left and bottom marker centres and pixels per reading, from the
/// raw readings at the panel edges measured on the board
const TOUCH_OFFSET: (u16, u16) = (353, 365);
const TOUCH_SCALE: (f32, f32) = (239.0 / 3618.0, 319.0 / 3573.0);

const BACKGROUND: Rgb565 = Rgb565::BLACK;
const FOREGROUND: Rgb565 = Rgb565::WHITE;
const ACCENT: Rgb565 = Rgb565::new(0, 31, 29);

const MARGIN: i32 = 10;
const BAR_TOP: i32 = 100;
const BAR_HEIGHT: u32 = 12;
const BUTTONS_TOP: i32 = 130;
const BUTTON_SIZE: u32 = 20;
const PAINT_TOP: i32 = 170;
const PAINT_SIZE: u32 = 6;
const LOOP_MS: u32 = 20;
const VCOM_TICKS: u32 = 50;

fn calibration() -> TouchCalibrationData {
    let mut bytes = [0; TouchCalibrationData::SERIALIZED_LEN];
    bytes[0..2].copy_from_slice(&TOUCH_OFFSET.0.to_le_bytes());
    bytes[2..4].copy_from_slice(&TOUCH_OFFSET.1.to_le_bytes());
    bytes[4..8].copy_from_slice(&TOUCH_SCALE.0.to_le_bytes());
    bytes[8..12].copy_from_slice(&TOUCH_SCALE.1.to_le_bytes());
    TouchCalibrationData::from_bytes(&bytes)
}

fn fill<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    colour: Rgb565,
) {
    let _ = Rectangle::new(Point::new(x, y), Size::new(w, h))
        .into_styled(PrimitiveStyle::with_fill(colour))
        .draw(target);
}

fn line<D: DrawTarget<Color = Rgb565>>(
    target: &mut D,
    y: i32,
    text: &str,
    style: MonoTextStyle<Rgb565>,
) {
    let _ = Text::new(text, Point::new(MARGIN, y), style).draw(target);
}

#[entry]
fn main() -> ! {
    let p = pac::Peripherals::take().unwrap();
    let mut board = Board::new(p);

    let _ = board.temperature.sensor.enable();
    let start = NaiveDate::from_ymd_opt(2026, 11, 8)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let _ = board.rtc.clock.set_datetime(&start);
    let _ = board.rtc.clock.enable();
    board.touch.calibrate_from_data(calibration());

    let title = MonoTextStyle::new(&FONT_10X20, ACCENT);
    let text = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(FOREGROUND)
        .background_color(BACKGROUND)
        .build();

    let display = &mut board.display;
    let _ = display.clear(BACKGROUND);
    line(display, 25, "APUS-RENODE", title);
    fill(display, 0, PAINT_TOP - 4, DISPLAY_WIDTH.into(), 1, ACCENT);

    let mut tick: u32 = 0;
    loop {
        let mut temperature: String<12> = String::new();
        let _ = match board.temperature.sensor.read_temperature() {
            Ok(celsius) => write!(temperature, "{:.1} C", celsius),
            Err(_) => temperature.write_str("error"),
        };
        let mut time: String<12> = String::new();
        let _ = match board.rtc.clock.datetime() {
            Ok(now) => write!(time, "{}", now.time()),
            Err(_) => time.write_str("error"),
        };
        let millivolts = board.potentiometer.read(&mut board.adc);

        let mut value: String<32> = String::new();
        let _ = write!(value, "Temperature   {:>8}", temperature);
        line(&mut board.display, 50, &value, text);
        value.clear();
        let _ = write!(value, "Time          {:>8}", time);
        line(&mut board.display, 66, &value, text);
        value.clear();
        let _ = write!(value, "Potentiometer {:>5} mV", millivolts);
        line(&mut board.display, 82, &value, text);

        if tick.is_multiple_of(VCOM_TICKS) {
            let _ = writeln!(
                board.vcom.serial,
                "{}  {}  {} mV
",
                temperature, time, millivolts
            );
        }

        let width = DISPLAY_WIDTH as u32 - 2 * MARGIN as u32;
        let filled = (u32::from(millivolts) * width / u32::from(MAX_VALUE)).min(width);
        fill(
            &mut board.display,
            MARGIN,
            BAR_TOP,
            filled,
            BAR_HEIGHT,
            ACCENT,
        );
        fill(
            &mut board.display,
            MARGIN + filled as i32,
            BAR_TOP,
            width - filled,
            BAR_HEIGHT,
            Rgb565::CSS_DIM_GRAY,
        );

        let pressed = [
            board.buttons.wake_up.pressed(),
            board.buttons.user_0.pressed(),
            board.buttons.user_1.pressed(),
        ];
        for (index, pressed) in pressed.into_iter().enumerate() {
            let led = match index {
                0 => &mut board.leds.led3 as &mut dyn LedLike,
                1 => &mut board.leds.led4,
                _ => &mut board.leds.led5,
            };
            if pressed {
                led.on();
            } else {
                led.off();
            }
            let colour = if pressed {
                ACCENT
            } else {
                Rgb565::CSS_DIM_GRAY
            };
            let x = MARGIN + index as i32 * (BUTTON_SIZE as i32 + MARGIN);
            fill(
                &mut board.display,
                x,
                BUTTONS_TOP,
                BUTTON_SIZE,
                BUTTON_SIZE,
                colour,
            );
        }

        if let Ok(Some(point)) = board.touch.read() {
            let y = i32::from(point.y);
            if y >= PAINT_TOP && y < i32::from(DISPLAY_HEIGHT) {
                fill(
                    &mut board.display,
                    i32::from(point.x),
                    y,
                    PAINT_SIZE,
                    PAINT_SIZE,
                    FOREGROUND,
                );
            }
        }

        board.delay.delay_ms(LOOP_MS);
        tick = tick.wrapping_add(1);
    }
}
