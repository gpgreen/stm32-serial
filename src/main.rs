#![no_std]
#![no_main]

// pick a panicking behavior
extern crate panic_halt; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                         // extern crate panic_abort; // requires nightly
                         // extern crate panic_itm; // logs messages over ITM; requires ITM support
                         // extern crate panic_semihosting; // logs messages to the host stderr; requires a debugger

use cortex_m_rt::entry;
use nb::block;
use nucleo_f103rb::hal::{prelude::*, stm32, timer::Timer};
use nucleo_f103rb::serial::Serial;
mod serial;
use serial_packet_parser::PacketParser;
mod register;
use register::Registers;

#[entry]
fn main() -> ! {
    // Get access to the core peripherals from the cortex-m crate
    let cp = cortex_m::Peripherals::take().unwrap();
    // Get access to the device specific peripherals from the peripheral access crate
    let dp = stm32::Peripherals::take().unwrap();

    // Take ownership over the raw flash and rcc devices and convert them into the corresponding
    // HAL structs
    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    // Freeze the configuration of all the clocks in the system and store
    // the frozen frequencies in `clocks`
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    // create registers
    let mut registers = Registers::new();
    
    // afio
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    // gpioa
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);

    // USART2 on Pins A2 and A3
    let pin_tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
    let pin_rx = gpioa.pa3;
    // Create an interface struct for USART2 with 115200 Baud
    let serial = Serial::usart2(
        dp.USART2,
        (pin_tx, pin_rx),
        &mut afio.mapr,
        115_200.bps(),
        clocks,
        &mut rcc.apb1,
    );

    // separate into tx and rx channels
    let (mut tx, mut rx) = serial.split();

    // Write 'R' to the USART
    block!(tx.write(b'R')).ok();

    // Configure gpio A pin 5 as a push-pull output. The `crh` register is passed to the function
    // in order to configure the port. For pins 0-7, crl should be passed instead.
    let mut led = gpioa.pa5.into_push_pull_output(&mut gpioa.crl);

    // Configure the syst timer to trigger an update every second
    let mut timer = Timer::syst(cp.SYST, 1.hz(), clocks);

    // create serial types
    let mut parser = PacketParser::new();
    let mut serial_handler = serial::SerialHandler::new();

    // Wait for the timer to trigger an update and change the state of the LED
    loop {
        block!(timer.wait()).unwrap();
        led.set_high();
        block!(timer.wait()).unwrap();
        led.set_low();
        // if there is a byte ready, fetch from the USART and store it in "received"
        match rx.read() {
            Ok(_) => {
                let received = block!(rx.read()).unwrap();
                parser = serial_handler.receive_byte(received, parser);
            }
            Err(nb::Error::WouldBlock) => {}
            _ => {}
        }
    }
}
