/// A simple tracing_subscriber implementation for no_std targets
use tracing::{
    dispatcher::Dispatch,
    field::{self, Field},
    span::{Attributes, Record},
    Event, Id, Metadata, Subscriber,
};

// - tracing_println ----------------------------------------------------------

#[macro_export]
macro_rules! tracing_println {
    ($($arg:tt)*) => {{
        // dummy logger
        #[cfg(not(any(feature="log-itm", feature="log-semihosting", feature="log-uart")))]
        {
            use ockam_core::compat::io::Write;
            let mut buffer = [0 as u8; 1];
            let mut cursor = ockam_core::compat::io::Cursor::new(&mut buffer[..]);

            match write!(&mut cursor, $($arg)*) {
                Ok(()) => (),
                Err(_) => (),
            }
        }

        // itm logger
        #[cfg(feature="log-itm")]
        {
            // give the itm buffer time to empty
            cortex_m::asm::delay(350_000);
            // print output using itm peripheral
            let itm = unsafe { &mut *cortex_m::peripheral::ITM::ptr() };
            cortex_m::iprintln!(&mut itm.stim[0], $($arg)*);
        }

        // semihosting logger
        #[cfg(feature="log-semihosting")]
        cortex_m_semihosting::hprintln!($($arg)*).unwrap();

        // uart logger
        #[cfg(all(feature="log-uart", feature="atsame54"))]
        {
            use atsame54_xpro as hal;
            use hal::prelude::_embedded_hal_serial_Write;
            cortex_m::asm::delay(500_000);
            crate::uart_println!($($arg)*);
        }
    }};
}

// - registration -------------------------------------------------------------

pub fn register() {
    let subscriber = EmbeddedSubscriber::new();
    let dispatch = Dispatch::new(subscriber);
    tracing::dispatcher::set_global_default(dispatch)
        .expect("global default dispatcher for tracing is already set");
    tracing::debug!("tracing_subscriber::register initialized");
}

#[cfg(feature = "log-uart")]
pub fn register_with_uart(uart: uart::Uart) {
    unsafe {
        uart::UART.replace(uart);
    }

    let subscriber = EmbeddedSubscriber::new();
    let dispatch = Dispatch::new(subscriber);
    tracing::dispatcher::set_global_default(dispatch)
        .expect("global default dispatcher for tracing is already set");
    tracing::debug!("tracing_subscriber::register_with_uart initialized");
}

// - EmbeddedSubscriber -------------------------------------------------------

// TODO add a SubscriberBuilder so we can set log levels etc.
//
// See: https://docs.rs/tracing-subscriber/0.3.3/tracing_subscriber/fmt/struct.Subscriber.html
//      tracing.git/tracing-subscriber/src/fmt/format/pretty.rs

struct EmbeddedSubscriber;

impl EmbeddedSubscriber {
    fn new() -> Self {
        Self
    }
}

impl Subscriber for EmbeddedSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        crate::tracing_println!("EmbeddedSubscriber::new_span");
        tracing::span::Id::from_u64(0xAAAA)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {
        crate::tracing_println!("EmbeddedSubscriber::record");
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {
        crate::tracing_println!("EmbeddedSubscriber::record_follows_from");
    }

    fn event(&self, event: &Event<'_>) {
        let mut visitor = EmbeddedVisitor::new();
        event.record(&mut visitor);
    }

    fn enter(&self, _span: &Id) {
        crate::tracing_println!("EmbeddedSubscriber::enter");
    }

    fn exit(&self, _span: &Id) {
        crate::tracing_println!("EmbeddedSubscriber::exit");
    }
}

// - EmbeddedVisitor ----------------------------------------------------------

struct EmbeddedVisitor;

impl EmbeddedVisitor {
    fn new() -> Self {
        Self
    }
}

impl field::Visit for EmbeddedVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            crate::tracing_println!("{}", value);
        } else {
            crate::tracing_println!("unknown: {}: {:?}", field.name(), value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn core::fmt::Debug) {
        if field.name() == "message" {
            crate::tracing_println!("{:?}", value);
        } else {
            crate::tracing_println!("unknown: {}: {:?}", field.name(), value);
        }
    }
}

// - Support for UART logging -------------------------------------------------

// TODO this needs to be generic across Uart

#[cfg(feature = "log-uart")]
use crate::tracing_subscriber::uart::BufferWriter;

#[cfg(feature = "log-uart")]
use core::fmt::Write;

#[cfg(all(feature = "log-uart", feature = "atsame54"))]
pub(crate) mod uart {
    use atsame54_xpro as hal;
    use hal::gpio::v2::{Pin, PA04, PA05};
    use hal::sercom::v2::{uart, IoSet3, Sercom0};

    type Rx = Pin<PA05, hal::gpio::v2::AlternateD>;
    type Tx = Pin<PA04, hal::gpio::v2::AlternateD>;
    type Pads = uart::Pads<Sercom0, IoSet3, Rx, Tx>;
    type Config = uart::Config<Pads, uart::EightBit>;

    pub(crate) type Uart = uart::Uart<Config, uart::Duplex>;

    pub(crate) static mut UART: Option<Uart> = None;

    #[macro_export]
    macro_rules! uart_println {
        ($($arg:tt)*) => {{
            if let Some(uart) = unsafe { uart::UART.as_mut() } {
                let mut buffer = [0u8; 512];
                let mut buffer = BufferWriter::new(&mut buffer[..]);
                writeln!(&mut buffer,  $($arg)*).unwrap();
                for byte in buffer.as_bytes() {
                    // NOTE `block!` blocks until `uart.write()` completes and returns
                    // `Result<(), Error>`
                    match ::nb::block!(uart.write(*byte)) {
                        Ok(()) => (),
                        Err(_) => {
                            cortex_m::asm::delay(10_000);
                        },
                    }
                }
            }
        }};
    }

    // - BufferWriter ---------------------------------------------------------

    pub struct BufferWriter<'a> {
        buffer: &'a mut [u8],
        cursor: usize,
    }

    #[allow(dead_code)]
    impl<'a> BufferWriter<'a> {
        pub fn new(buffer: &'a mut [u8]) -> Self {
            BufferWriter { buffer, cursor: 0 }
        }

        pub fn reset(&mut self) {
            self.cursor = 0;
        }

        pub fn as_bytes(&self) -> &[u8] {
            &self.buffer[0..self.cursor]
        }

        pub fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buffer[0..self.cursor]).unwrap()
        }
    }

    impl core::fmt::Write for BufferWriter<'_> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let len = self.buffer.len();
            for (i, &b) in self.buffer[self.cursor..len]
                .iter_mut()
                .zip(s.as_bytes().iter())
            {
                *i = b;
            }
            self.cursor = usize::min(len, self.cursor + s.as_bytes().len());
            Ok(())
        }
    }
}
