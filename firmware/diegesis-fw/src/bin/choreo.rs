#![no_main]
#![no_std]

use choreographer::engine::Behavior;
use choreographer::engine::Sequence;
use diegesis_fw::patterns::Direction;
use diegesis_fw::patterns::{color_walker, boot_seq, rainbow_crawler};
use diegesis_fw::{
    self as _, // global logger + panicking-behavior + memory layout
    groundhog_nrf52::GlobalRollingTimer
};
use groundhog::RollingTimer;
use nrf52840_hal::{
    clocks::Clocks,
    gpio::{p0::Parts as P0Parts, p1::Parts as P1Parts, Level},
    pac,
};
use nrf_smartled::pwm::Pwm;
use smart_leds::colors::BLUE;
use smart_leds::{colors, gamma, brightness, SmartLedsWrite};

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello, world!");

    let board = pac::Peripherals::take().unwrap();

    // Enable instruction caches for MAXIMUM SPEED
    board.NVMC.icachecnf.write(|w| w.cacheen().set_bit());
    cortex_m::asm::isb();

    defmt::info!("Hello, world!");

    while !board
        .POWER
        .usbregstatus
        .read()
        .vbusdetect()
        .is_vbus_present()
    {}

    // wait until USB 3.3V supply is stable
    while !board
        .POWER
        .events_usbpwrrdy
        .read()
        .events_usbpwrrdy()
        .bit_is_clear()
    {}

    let clocks = Clocks::new(board.CLOCK);
    let _clocks = clocks.enable_ext_hfosc();

    GlobalRollingTimer::init(board.TIMER0);
    let gpios_p0 = P0Parts::new(board.P0);
    let gpios_p1 = P1Parts::new(board.P1);

    let _start_stop_btn = gpios_p1.p1_15.into_pulldown_input().degrade();
    let _start_stop_led = gpios_p1.p1_14.into_push_pull_output(Level::Low).degrade();

    // Enable smartled switch
    let _ = gpios_p0.p0_06.into_push_pull_output(Level::Low);

    let mut data = [colors::BLACK; 10];
    let mut led = Pwm::new(board.PWM0, gpios_p0.p0_13.degrade());

    led.write(gamma(data.iter().cloned())).ok();
    let timer = GlobalRollingTimer::new();

    let mut script: [Sequence<GlobalRollingTimer, 8>; 10] = Sequence::new_array();

    // Update the screen
    let pulse = timer.get_ticks();
    boot_seq(
        &mut script,
        Behavior::OneShot,
        Direction::Clockwise,
    );

    let reset_clock = timer.get_ticks();
    let mut oneshot = false;
    let mut twoshot = false;

    loop {
        let start = timer.get_ticks();
        let mut any = false;

        if !twoshot && timer.millis_since(reset_clock) >= 5000 {
            color_walker(
                &mut script,
                colors::CORNFLOWER_BLUE,
                Behavior::LoopForever,
                Direction::Clockwise
            );
            twoshot = true;
        }

        let upd_start = timer.get_ticks();
        for (led, scr) in data.iter_mut().zip(script.iter_mut()) {
            if let Some(pix) = scr.poll() {
                any |= *led != pix;
                *led = pix;
            }
        }
        let update_time = timer.micros_since(upd_start);
        let disp_start = timer.get_ticks();

        if any {
            led.write(brightness(gamma(data.iter().cloned()), 32)).unwrap();
            let display_time = timer.micros_since(disp_start);
            defmt::info!("Render: {=u32:}us Display: {=u32:}us", update_time, display_time);
        }


        while timer.millis_since(start) < 16 { }
    }
}
