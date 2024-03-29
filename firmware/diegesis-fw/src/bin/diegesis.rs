#![no_main]
#![no_std]

use core::sync::atomic::{AtomicBool, Ordering};

use diegesis_fw::{Board, InternalReport, groundhog_nrf52::GlobalRollingTimer, pinmap::{PinMap, Leds}, profiler, saadc_src::SaadcSrc, spim_src::SpimSrc, time_ticks};
use nrf52840_hal::{
    clocks::{Clocks, ExternalOscillator, Internal, LfOscStopped},
    gpio::{
        p0::{Parts as P0Parts, P0_02, P0_03, P0_29},
        p1::Parts as P1Parts,
        Disconnected, Level, Output, Pin, PushPull,
    },
    pac::{Interrupt, SPIM0, SPIM1, SPIM2, SPIM3, TIMER1},
    ppi::{self, Ppi0, Ppi1},
    spim::Frequency,
    usbd::Usbd,
};

use bbqueue::{consts as bbconsts, BBBuffer, ConstBBBuffer};
use embedded_hal::digital::v2::OutputPin;
use groundhog::RollingTimer;
use heapless::{mpmc::MpMcQueue, pool::singleton::Pool};
use rtic::app;
use usb_device::{bus::UsbBusAllocator, class::UsbClass as _, device::UsbDeviceState, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

type UsbDevice<'a> = usb_device::device::UsbDevice<'static, Usbd<'a>>;
type UsbSerial<'a> = SerialPort<'static, Usbd<'a>>;

#[allow(non_camel_case_types)]
pub mod allocs {
    use heapless::pool;
    pool!(DIGITAL_POOL: [u8; 4096]);
    pool!(ANALOG_POOL: [i16; 2048]);
}

static ENCODED_QUEUE: BBBuffer<bbconsts::U32768> = BBBuffer(ConstBBBuffer::new());
static POOL_QUEUE: MpMcQueue<InternalReport<allocs::DIGITAL_POOL, allocs::ANALOG_POOL>, 32> =
    MpMcQueue::new();
static PROFILER: Profiler = Profiler::new();
static FUSE: AtomicBool = AtomicBool::new(true);

profiler!(Profiler {
    spim_p0_ints,
    spim_p1_ints,
    spim_p2_ints,
    spim_p3_ints,
    saadc_ints,
    usb_writes,
    report_sers,
    encoded_in_bytes,
    bbq_push_bytes,
    bbq_pull_bytes,
    idle_loop_iters,

    ticks_usb,
    ticks_misc,
    ticks_encoding,
    ticks_draining,

    ticks_spimp0,
    ticks_spimp1,
    ticks_spimp2,
    ticks_spimp3,

    ticks_saadc
} => ProfilerRpt);

// TODO: Replace with "Active" and "Inactive" instead of High/Low
#[derive(Clone, Copy, Debug)]
enum ButtonDebounce {
    StableLow,
    StableHigh,
    MaybeLow(u32),
    MaybeHigh(u32),
}

impl ButtonDebounce {
    fn poll(&mut self, is_low: bool) -> Option<Level> {
        let timer = GlobalRollingTimer::new();
        let mut retval = None;

        *self = match *self {
            ButtonDebounce::StableHigh if is_low => ButtonDebounce::MaybeLow(timer.get_ticks()),
            ButtonDebounce::StableLow if !is_low => ButtonDebounce::MaybeHigh(timer.get_ticks()),
            ButtonDebounce::MaybeHigh(_) if is_low => ButtonDebounce::StableLow,
            ButtonDebounce::MaybeHigh(start) if timer.millis_since(start) >= 5 => {
                retval = Some(Level::High);
                ButtonDebounce::StableHigh
            }
            ButtonDebounce::MaybeLow(_) if !is_low => ButtonDebounce::StableHigh,
            ButtonDebounce::MaybeLow(start) if timer.millis_since(start) >= 5 => {
                retval = Some(Level::Low);
                ButtonDebounce::StableLow
            }
            retain => retain,
        };

        retval
    }
}

#[app(device = nrf52840_hal::pac, peripherals = true)]
const APP: () = {
    struct Resources {
        usb_dev: UsbDevice<'static>,
        serial: UsbSerial<'static>,
        spim_p0: SpimSrc<SPIM0, allocs::DIGITAL_POOL, allocs::ANALOG_POOL, 32>,
        spim_p1: SpimSrc<SPIM1, allocs::DIGITAL_POOL, allocs::ANALOG_POOL, 32>,
        spim_p2: SpimSrc<SPIM2, allocs::DIGITAL_POOL, allocs::ANALOG_POOL, 32>,
        spim_p3: SpimSrc<SPIM3, allocs::DIGITAL_POOL, allocs::ANALOG_POOL, 32>,
        saadc: SaadcSrc<
            (P0_02<Disconnected>, P0_03<Disconnected>, P0_29<Disconnected>),
            TIMER1,
            allocs::ANALOG_POOL,
            allocs::DIGITAL_POOL,
            Ppi0,
            Ppi1,
            32,
        >,
        start_stop_btn: <Board as PinMap>::ButtonPin,
        start_stop_led: Option<Pin<Output<PushPull>>>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        static mut CLOCKS: Option<Clocks<ExternalOscillator, Internal, LfOscStopped>> = None;
        static mut USB_BUS: Option<UsbBusAllocator<Usbd<'static>>> = None;
        static mut DATA_POOL_A: [u8; 24 * 4096] = [0u8; 24 * 4096];
        static mut DATA_POOL_B: [u8; 16 * 4096] = [0u8; 16 * 4096];

        // Enable instruction caches for MAXIMUM SPEED
        let board = ctx.device;
        board.NVMC.icachecnf.write(|w| w.cacheen().set_bit());
        cortex_m::asm::isb();

        // NOTE: UPDATE WITH CORRECT PAGE COUNTS
        allocs::DIGITAL_POOL::grow(DATA_POOL_A);
        allocs::ANALOG_POOL::grow(DATA_POOL_B);

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
        let clocks = clocks.enable_ext_hfosc();

        GlobalRollingTimer::init(board.TIMER0);
        let usbd = board.USBD;

        let gpios_p0 = P0Parts::new(board.P0);
        let gpios_p1 = P1Parts::new(board.P1);
        let pins = Board::map_pins(gpios_p0, gpios_p1);

        let spim0 = SpimSrc::from_parts(
            board.SPIM0,
            pins.spim_p0_data,
            pins.spim_p0_clk,
            &POOL_QUEUE,
            Frequency::M2,
            GlobalRollingTimer,
            0,
        );

        let spim1 = SpimSrc::from_parts(
            board.SPIM1,
            pins.spim_p1_data,
            pins.spim_p1_clk,
            &POOL_QUEUE,
            Frequency::M2,
            GlobalRollingTimer,
            1,
        );

        let spim2 = SpimSrc::from_parts(
            board.SPIM2,
            pins.spim_p2_data,
            pins.spim_p2_clk,
            &POOL_QUEUE,
            Frequency::M2,
            GlobalRollingTimer,
            2,
        );

        let spim3 = SpimSrc::from_parts(
            board.SPIM3,
            pins.spim_p3_data,
            pins.spim_p3_clk,
            &POOL_QUEUE,
            Frequency::M2,
            GlobalRollingTimer,
            3,
        );

        let ppi = ppi::Parts::new(board.PPI);
        let saadc = SaadcSrc::new(
            board.SAADC,
            board.TIMER1,
            pins.adcs,
            ppi.ppi0,
            ppi.ppi1,
            &POOL_QUEUE,
        );

        let start_stop_btn = Board::into_button(pins.start_pause_btn);
        let start_stop_led = if let Leds::DiscreteLeds { led3, .. } = pins.leds {
            Some(led3.into_push_pull_output(Level::High))
        } else {
            None
        };

        *CLOCKS = Some(clocks);
        let clocks = CLOCKS.as_ref().unwrap();
        *USB_BUS = Some(Usbd::new(usbd, &clocks));
        let usb_bus = USB_BUS.as_ref().unwrap();

        let serial = SerialPort::new(usb_bus);
        let usb_dev = UsbDeviceBuilder::new(usb_bus, UsbVidPid(0x16c0, 0x27DD))
            .manufacturer("Ferrous Systems")
            .product("diegesis")
            .serial_number("diegesis-001")
            .device_class(USB_CLASS_CDC)
            .max_packet_size_0(64) // (makes control transfers 8x faster)
            .build();

        init::LateResources {
            usb_dev,
            serial,
            spim_p0: spim0,
            spim_p1: spim1,
            spim_p2: spim2,
            spim_p3: spim3,
            saadc,

            start_stop_btn,
            start_stop_led,
        }
    }

    #[task(binds = SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0, resources = [spim_p0])]
    fn spim_p0(c: spim_p0::Context) {
        PROFILER.spim_p0_ints();
        time_ticks!(PROFILER.ticks_spimp0, {
            c.resources.spim_p0.poll(&FUSE);
        });
    }

    #[task(binds = SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1, resources = [spim_p1])]
    fn spim_p1(c: spim_p1::Context) {
        PROFILER.spim_p1_ints();
        time_ticks!(PROFILER.ticks_spimp1, {
            c.resources.spim_p1.poll(&FUSE);
        });
    }

    #[task(binds = SPIM2_SPIS2_SPI2, resources = [spim_p2])]
    fn spim_p2(c: spim_p2::Context) {
        PROFILER.spim_p2_ints();
        time_ticks!(PROFILER.ticks_spimp2, {
            c.resources.spim_p2.poll(&FUSE);
        });
    }

    #[task(binds = SPIM3, resources = [spim_p3])]
    fn spim_p3(c: spim_p3::Context) {
        PROFILER.spim_p3_ints();
        time_ticks!(PROFILER.ticks_spimp3, {
            c.resources.spim_p3.poll(&FUSE);
        });
    }

    #[task(binds = SAADC, resources = [saadc])]
    fn saadc(c: saadc::Context) {
        PROFILER.saadc_ints();
        time_ticks!(PROFILER.ticks_saadc, {
            c.resources.saadc.poll(&FUSE);
        });
    }

    #[idle(resources = [usb_dev, serial, start_stop_btn, start_stop_led])]
    fn idle(mut c: idle::Context) -> ! {
        let mut state: UsbDeviceState = UsbDeviceState::Default;
        let timer = GlobalRollingTimer::new();
        let (mut enc_prod, mut enc_cons) = ENCODED_QUEUE.try_split().unwrap();

        let start = timer.get_ticks();
        let mut last_profile = start;
        let mut fuse_timeout = None;

        let mut button = ButtonDebounce::StableHigh;
        let mut running = false;

        let mut last_loop = timer.get_ticks();
        let mut min_ticks = 0xFFFFFFFF;
        let mut max_ticks = 0x00000000;

        let mut temp_buf = [0u8; 4096 + 1024];

        loop {
            let elapsed = timer.ticks_since(last_loop);
            min_ticks = min_ticks.min(elapsed);
            max_ticks = max_ticks.max(elapsed);
            last_loop = timer.get_ticks();

            PROFILER.idle_loop_iters();
            let new_state = c.resources.usb_dev.state();
            if new_state != state {
                defmt::info!("State change!");
                state = new_state;

                if new_state == UsbDeviceState::Configured {
                    defmt::info!("Configured!");
                }
            }

            let usb_d = &mut c.resources.usb_dev;
            let serial = &mut c.resources.serial;

            // TODO: In the current version of nrf-usb, we need to poll the USB once
            // per write. This is why the following code is round-robin. In the future,
            // when a fix for this is available, we may re-consider true round-robin.
            time_ticks!(PROFILER.ticks_usb, {
                usb_poll(usb_d, serial);
            });

            if state != UsbDeviceState::Configured {
                continue;
            }

            /////////////////////////////////////////////////////////
            // FUSES, START, AND STOP
            /////////////////////////////////////////////////////////
            time_ticks!(PROFILER.ticks_misc, {
                let is_active = Board::button_active(c.resources.start_stop_btn);
                if let Some(Level::Low) = button.poll(is_active) {
                    if running {
                        // Stopping by blowing the fuse
                        defmt::info!("Stopping!");
                        FUSE.store(true, Ordering::SeqCst);
                        if let Some(led) = c.resources.start_stop_led.as_mut() {
                            led.set_high().ok();
                        }
                        running = false;
                    } else if fuse_timeout.is_some() {
                        // TODO: start after the fuse is cleared?
                        defmt::info!("Not starting, waiting for fuse!");
                    } else {
                        defmt::info!("Starting!");
                        FUSE.store(false, Ordering::SeqCst);
                        rtic::pend(Interrupt::SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0);
                        rtic::pend(Interrupt::SPIM1_SPIS1_TWIM1_TWIS1_SPI1_TWI1);
                        rtic::pend(Interrupt::SPIM2_SPIS2_SPI2);
                        rtic::pend(Interrupt::SPIM3);
                        rtic::pend(Interrupt::SAADC);

                        if let Some(led) = c.resources.start_stop_led.as_mut() {
                            led.set_low().ok();
                        }
                        running = true;
                    }
                }

                if let Some(tick) = fuse_timeout.take() {
                    if timer.millis_since(tick) > 2500 {
                        defmt::info!("Fuse restored! Cleared");
                        // NOTE: DON'T auto-clear the fuse! wait for an explicit run command
                    } else {
                        // Still cooling...
                        fuse_timeout = Some(tick);
                    }
                }

                if running && fuse_timeout.is_none() && FUSE.load(Ordering::SeqCst) {
                    defmt::info!("Fuse blown! Cooling down...");
                    if let Some(led) = c.resources.start_stop_led.as_mut() {
                        led.set_high().ok();
                    }
                    fuse_timeout = Some(timer.get_ticks());
                    running = false;
                }

                if timer.millis_since(last_profile) >= 1000 {
                    let rpt = PROFILER.clear_and_report();
                    defmt::info!("{}", rpt);

                    last_profile = timer.get_ticks();

                    defmt::info!(
                        "min: {}, max: {}, avg: {}",
                        min_ticks,
                        max_ticks,
                        (4_000_000 / rpt.idle_loop_iters),
                    );

                    min_ticks = 0xFFFFFFFF;
                    max_ticks = 0x00000000;
                }
            });

            // TODO: read?

            // TODO: with a little more complexity, we could use split grants
            // for a more efficient use of the encoding buffer. For now, we may
            // end up wasting 0 <= n < 5KiB at the end of the ring, which is a
            // whole pbox worth (7.8% of 64K capacity)
            if let Ok(mut wgr) = enc_prod.grant_exact(1024 + 4096) {
                if let Some(mut new_rpt) = POOL_QUEUE.dequeue() {
                    time_ticks!(PROFILER.ticks_encoding, {
                        PROFILER.report_sers();

                        let report = new_rpt.as_data_report();
                        let serialized = postcard::to_slice(&report, &mut temp_buf).unwrap();

                        let len = kolben::rlercobs::encode_all(serialized, &mut wgr, true).unwrap().len();
                        wgr.commit(len);

                        PROFILER
                            .bbq_push_bytes
                            .fetch_add(len as u32, Ordering::SeqCst);
                    });
                }
            };

            // Second: Drain bytes into the serial port in order to
            // free up space to encode more.
            if let Ok(rgr) = enc_cons.read() {
                time_ticks!(PROFILER.ticks_draining, {
                    match serial.write(&rgr) {
                        Ok(n) => {
                            PROFILER.usb_writes();
                            PROFILER
                                .bbq_pull_bytes
                                .fetch_add(n as u32, Ordering::SeqCst);
                            rgr.release(n);
                        }
                        Err(UsbError::WouldBlock) => {
                            rgr.release(0);
                        }
                        Err(e) => {
                            rgr.release(0);
                            panic!("BAD USB WRITE - {:?}", e);
                        }
                    }
                });
            }
        }
    }
};

fn usb_poll(usb_dev: &mut UsbDevice, serial: &mut UsbSerial) {
    if usb_dev.poll(&mut [serial]) {
        serial.poll();
    }
}
