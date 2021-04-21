#![feature(default_alloc_error_handler)]
#![no_main]
#![no_std]

use log::LevelFilter;
mod rttlogger;

use alloc_cortex_m::CortexMHeap;

use rtt_target::rtt_init_print;

use panic_rtt_target as _;
// use panic_semihosting as _;

use core::{
    future,
    sync::atomic::{AtomicU32, Ordering},
};

use cortex_m::asm;
// use embedded_hal::digital::v2::{OutputPin as _, StatefulOutputPin};
use hal::prelude::*;

use hal::{
    clocks::{self, Clocks},
    gpio::{p0, Input, Level, Output, Pin, PullUp, PushPull},
    gpiote::{Gpiote, GpioteChannel, GpioteChannelEvent, TaskOutPolarity},
    pac::{self, interrupt, Interrupt, NVIC_PRIO_BITS, RTC0},
    rtc::{Rtc, RtcInterrupt},
    Timer,
};

use core::future::Future;
use core::task::Context;
use core::task::Poll;

use cortex_m_async::flag::Flag;

mod worker;
use worker::*;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[cortex_m_rt::pre_init]
unsafe fn preinit() {
    // globals do not exist here just yet
    // as such we cannot setup the heap here
}

const HEAP_SIZE: usize = 2048;

fn setup_heap() {
    let heap_start = cortex_m_rt::heap_start() as usize;
    let heap_end = heap_start + HEAP_SIZE;
    let heap_size = heap_end - heap_start;
    unsafe {
        ALLOCATOR.init(heap_start, heap_size);
    }
    log::info!("Heap ranges from 0x{:x}..0x{:x}", heap_start, heap_end);
    // add a stack canary to avoid stack growth into heap
}

struct Led {
    inner: Pin<Output<PushPull>>,
}

impl Led {
    fn new(pin: Pin<Output<PushPull>>) -> Self {
        Self { inner: pin }
    }
    fn on(&mut self) {
        let _ = self.inner.set_low();
    }
    fn off(&mut self) {
        let _ = self.inner.set_high();
    }
    fn is_on(&self) -> bool {
        self.inner.is_set_low().unwrap_or_default()
    }
    fn toggle(&mut self) {
        if self.is_on() {
            self.off()
        } else {
            self.on()
        }
    }
}

struct Button {
    pin: Pin<Input<PullUp>>,
    // not right, not pretty
    gpiote: hal::gpiote::Gpiote,
}

impl Button {
    fn new(gpiote: hal::gpiote::Gpiote, button: Pin<Input<PullUp>>) -> Self {
        Self {
            pin: button,
            gpiote,
        }
    }

    #[allow(unused)]
    fn enable_interrupt(&self) {
        self.gpiote
            .channel0()
            .input_pin(&self.pin)
            .hi_to_lo()
            .enable_interrupt();
    }

    #[allow(unused)]
    fn disable_interrupt(&self) {
        self.gpiote
            .channel0()
            .input_pin(&self.pin)
            .hi_to_lo()
            .disable_interrupt();
    }

    #[allow(unused)]
    fn clear_interrupt(&self) {
        self.gpiote.channel0().reset_events();
    }
}

struct Board {
    led1: Led,
    led2: Led,
    led3: Led,
    led4: Led,
    button1: Button,
    temp: hal::Temp,
    rtc: Rtc<RTC0>,
}

impl Board {
    fn init() -> Self {
        rtt_init_print!(BlockIfFull, 16384);

        if log::max_level() == LevelFilter::Off {
            log::set_max_level(LevelFilter::Info)
        }
        log::set_logger(&rttlogger::RttLogger).unwrap();

        setup_heap();

        let periph = hal::pac::Peripherals::take().expect("Is only called once");
        let pins = p0::Parts::new(periph.P0);

        let mut led1 = Led::new(pins.p0_13.degrade().into_push_pull_output(Level::High));
        let mut led2 = Led::new(pins.p0_14.degrade().into_push_pull_output(Level::High));
        let mut led3 = Led::new(pins.p0_15.degrade().into_push_pull_output(Level::High));
        let mut led4 = Led::new(pins.p0_16.degrade().into_push_pull_output(Level::High));

        led1.on();
        led2.on();
        led3.off();
        led4.off();
        let clocks = Clocks::new(periph.CLOCK);
        let clocks = clocks.enable_ext_hfosc();
        let clocks = clocks.set_lfclk_src_external(clocks::LfOscConfiguration::NoExternalNoBypass);
        let clocks = clocks.start_lfclk();
        let _clocks = clocks.enable_ext_hfosc();

        let mut core = cortex_m::Peripherals::take().expect("This is only called once. qed");

        // Creates a new RTC peripheral instance with a 12 bits prescaler. `f_RTC = 32_768 / (prescaler + 1 ) Hz`
        //
        // for a cycle time of 10s we need `prescaler = 32_767` which does not fit in 12 bits.
        // so just use a smaller (i.e. 2048 - 1 and count 16 overflows, or 4096 - 1 (which just fits in 12 bits) and 8 overflows)
        let mut rtc = Rtc::new(periph.RTC0, 4096 - 1).expect("Prescaler 2048 is fine by docs. qed");
        rtc.enable_interrupt(RtcInterrupt::Tick, Some(&mut core.NVIC));
        rtc.enable_interrupt(RtcInterrupt::Overflow, Some(&mut core.NVIC));
        rtc.enable_counter();

        let button1 = pins.p0_11.degrade().into_pullup_input();

        let gpiote = hal::gpiote::Gpiote::new(periph.GPIOTE);
        let button1 = Button::new(gpiote, button1);
        button1.enable_interrupt();

        let temp = hal::Temp::new(periph.TEMP);

        // there should be a better way to achieve this
        unsafe {
            cortex_m::peripheral::NVIC::unmask(interrupt::GPIOTE);
        }
        let board = Board {
            button1,
            temp,
            rtc,
            led1,
            led2,
            led3,
            led4,
        };

        board
    }
}

#[no_mangle]
fn main() -> ! {
    setup_heap();

    let Board {
        mut temp,
        mut led1,
        mut led2,
        mut led3,
        mut led4,
        ..
    } = Board::init();

    led1.on(); // tick
    led2.off(); // tack

    led3.on(); // alive check and toggle on button press

    led4.off(); // toggle on wfi enter

    let mut node = Node::new();

    let mut worker = ockam::worker::with(&node, TempPrintWorker {});
    let starting = worker.address("celsius").start();

    direct_executor::run_with_wake(
        async move {
            if let Some(address) = starting.await {
                loop {
                    match futures::future::select(ButtonPress::new(), OneHertz::new()).await {
                        futures::future::Either::Left(_) => {
                            log::info!("Button 1 pressed.");
                            led3.toggle();
                        }
                        futures::future::Either::Right(_) => {
                            log::info!("1 Hz.");
                            led1.toggle();
                            led2.toggle();
                        }
                    }

                    let celsius: i32 = temp.measure().to_num();
                    let kelvin: u32 = (celsius + 273_i32) as u32;

                    node.send(
                        &address,
                        Temperature {
                            kelvin,
                            time_since_boot: 0_u64,
                        },
                    );

                    log::debug!(" {}°C", celsius);
                }
            }
        },
        move || {
            led4.toggle();
            log::info!("💤");
            asm::wfi();
        },
        || {
            // IRQs are a HW feature
            // which in our case already set
            // the `Flag` so there is no additional wake
            // logic required.
        },
    );

    loop {
        asm::bkpt();
    }
}

// Force a backtrace on exit
pub fn exit() -> ! {
    log::info!(" ~ 👋 ~ ");

    core::sync::atomic::compiler_fence(Ordering::SeqCst);

    loop {
        asm::bkpt()
    }
}

static OVERFLOWS: AtomicU32 = AtomicU32::new(0);
static TICKS: AtomicU32 = AtomicU32::new(0);

#[interrupt]
fn RTC0() {
    let rtc = unsafe { core::mem::transmute::<_, pac::RTC0>(()) };

    let is_overflow = rtc.events_ovrflw.read().bits() != 0;
    if is_overflow {
        rtc.events_ovrflw.reset()
    }

    let is_tick = rtc.events_tick.read().bits() != 0;
    if is_tick {
        rtc.events_tick.reset()
    }

    drop(rtc);

    if is_overflow {
        let curr = OVERFLOWS.load(Ordering::Relaxed);
        OVERFLOWS.store(curr + 1, Ordering::Relaxed);
    }
    if is_tick {
        let curr = (TICKS.load(Ordering::Relaxed) + 1) & 0x07;
        TICKS.store(curr, Ordering::Relaxed);
        // 8 Tick irqs make a second
        if curr == 0 {
            OneHertz::notify_flag().wake()
        }
    }
}

struct OneHertz;

impl OneHertz {
    /// Flag can hold the _token of button toggle_
    /// which can be consumed by exactly one entity.
    fn notify_flag() -> &'static Flag {
        static FLAG: Flag = Flag::new();
        &FLAG
    }

    /// Create a new `button press future`.
    pub fn new() -> Self {
        Self
    }
}

impl Future for OneHertz {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Self::notify_flag().check(cx.waker()) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

#[interrupt]
fn GPIOTE() {
    // a bit too much, but for now this is good enough
    unsafe {
        core::mem::transmute::<_, pac::GPIOTE>(())
            .events_in
            .iter()
            .for_each(|x| x.reset())
    }

    ButtonPress::notify_flag().wake();
}

/// Not re-entrant, only one instance at a time will be triggered
// since the first one consumes the shared flag
struct ButtonPress;

impl ButtonPress {
    /// Flag can hold the _token of button toggle_
    /// which can be consumed by exactly one entity.
    fn notify_flag() -> &'static Flag {
        static FLAG: Flag = Flag::new();
        &FLAG
    }
    /// Create a new `button press future`.
    pub fn new() -> Self {
        Self
    }
}

impl Future for ButtonPress {
    type Output = ();
    fn poll(self: core::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Self::notify_flag().check(cx.waker()) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
