//! Based on an example from https://github.com/jamwaffles/ssd1306
//! 
//! Ported to STMF411
//! 
//! Constantly update a counter and display it as elapsed time
//! 
//! 
//! This example is for the STM32F411CEU6 board board using I2C1.
//!
//! Wiring connections are as follows for a 128x32 unbranded display:
//!
//! ```
//! Display -> Board
//! GND -> GND
//! +3.3V -> VCC
//! SDA -> PB9
//! SCL -> PB8
//! ```
//!
//! Best results when using `--release`.

#![no_std]
#![no_main]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32f4xx_hal as hal;
extern crate stm32f4;

use cortex_m_rt::entry;
use cortex_m::interrupt::{Mutex, free};
use cortex_m::peripheral::Peripherals as c_m_Peripherals;

use core::fmt;
use core::fmt::Write;
use arrayvec::ArrayString;

use core::ops::DerefMut;
use core::cell::{Cell, RefCell};

use stm32f4::stm32f411::interrupt;

use ssd1306::{prelude::*, Builder as SSD1306Builder};
use ssd1306::{mode::displaymode::DisplayModeTrait, prelude::*, Builder};

use crate::hal::{
    prelude::*,
    rcc::{Rcc, Clocks},
    i2c::I2c,
    stm32,
    timer::{Timer, Event},
    delay::Delay,
    stm32::Interrupt,
  
};

static ELAPSED: Mutex<Cell<u32>> = Mutex::new(Cell::new(0u32));
static TIMER_TIM2: Mutex<RefCell<Option<Timer<stm32::TIM2>>>> = Mutex::new(RefCell::new(None));


#[entry]
fn main() -> ! {
    if let (Some(dp), Some(cp)) = (
        stm32::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        // Set up the system clock. We want to run at 48MHz for this one.
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

        // Set up I2C - SCL is PB8 and SDA is PB9; they are set to Alternate Function 4, open drain
        
        let gpiob = dp.GPIOB.split();
        let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
        let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
        let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

        
        // Set up the display: using terminal mode with 128x32 display
        
        let mut disp: TerminalMode<_> = SSD1306Builder::new().size(DisplaySize::Display128x32).connect_i2c(i2c).into();
        
        disp.init().unwrap();

        disp.clear().unwrap();

        // set up delay provider

        let mut delay = Delay::new(cp.SYST, clocks);

        // set up timer and interrupt

        let mut timer = Timer::tim2(dp.TIM2, 1.hz(), clocks);
        timer.listen(Event::TimeOut);
        free(|cs| *TIMER_TIM2.borrow(cs).borrow_mut() = Some(timer));

        stm32::NVIC::unpend(Interrupt::TIM2);
        unsafe { stm32::NVIC::unmask(Interrupt::TIM2); };


        
        loop {

            // display is refreshed every 200 ms
            
            let mut buffer = ArrayString::<[u8; 64]>::new();

            let elapsed = free(|cs| ELAPSED.borrow(cs).get()); 

            let hours: u32 = elapsed / 3600;

            let minutes: u32 = elapsed / 60;

            let seconds: u32 = elapsed % 60;
            
            format_time(&mut buffer, hours as u8, minutes as u8, seconds as u8);
            
            disp.write_str(buffer.as_str());
            
            delay.delay_ms(200_u16);

        }
        
        
    }

    loop {}
}

#[interrupt]

// the ELAPSED value gets updated every second when the interrupt fires

fn TIM2() {

     
    free(|cs| {
        stm32::NVIC::unpend(Interrupt::TIM2);
        if let Some(ref mut tim2) = TIMER_TIM2.borrow(cs).borrow_mut().deref_mut() {
            tim2.clear_interrupt(Event::TimeOut);
        }

        ELAPSED.borrow(cs).set(ELAPSED.borrow(cs).get() + 1);

        
    });

    
}

fn format_time(buf: &mut ArrayString<[u8; 64]>, hours: u8, minutes: u8, seconds: u8) {
    fmt::write(buf, format_args!("    {:02}:{:02}:{:02}                                                    ", hours, minutes, seconds)).unwrap();
}

