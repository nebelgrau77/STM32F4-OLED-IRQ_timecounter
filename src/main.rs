//! Based on an example from https://github.com/jamwaffles/ssd1306
//! 
//! Ported to STMF411
//! 
//! Constantly update a counter and display it as elapsed time
//! //! 
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
//! LED: 
//! anode -> PA1
//! cathode -> GND through a 220 Ohm (or bigger) resistor
//! 
//! BUTTON: built-in button on PA0
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
    gpio::{gpioa::PA0, Edge, ExtiPin, Input, PullUp},
    i2c::I2c,
    stm32,
    timer::{Timer, Event},
    delay::Delay,
    time::{Hertz, MilliSeconds},
    stm32::{Interrupt,EXTI},
        
};

static ELAPSED: Mutex<Cell<u32>> = Mutex::new(Cell::new(0u32));
static TIMER_TIM2: Mutex<RefCell<Option<Timer<stm32::TIM2>>>> = Mutex::new(RefCell::new(None));

static SET: Mutex<Cell<u32>> = Mutex::new(Cell::new(0u32));
static BUTTON: Mutex<RefCell<Option<PA0<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));

static EXTI: Mutex<RefCell<Option<EXTI>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    if let (Some(mut dp), Some(cp)) = (
        stm32::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        // Set up the system clock. We want to run at 48MHz for this one.
        
        dp.RCC.apb2enr.write(|w| w.syscfgen().enabled());

        let rcc = dp.RCC.constrain();

        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

        // Set up I2C - SCL is PB8 and SDA is PB9; they are set to Alternate Function 4, open drain
        
        let gpiob = dp.GPIOB.split();
        let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
        let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
        let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

        //set up LED

        let gpioa = dp.GPIOA.split();
        let mut yellow = gpioa.pa1.into_push_pull_output();
        
        //set up the on-board button

        let mut board_btn = gpioa.pa0.into_pull_up_input();
        board_btn.make_interrupt_source(&mut dp.SYSCFG);
        board_btn.enable_interrupt(&mut dp.EXTI);
        board_btn.trigger_on_edge(&mut dp.EXTI, Edge::FALLING);
                
        // Set up the display: using terminal mode with 128x32 display
        
        let mut disp: TerminalMode<_> = SSD1306Builder::new().size(DisplaySize::Display128x32).connect_i2c(i2c).into();
        
        disp.init().unwrap();

        disp.clear().unwrap();

        // set up delay provider

        let mut delay = Delay::new(cp.SYST, clocks);

        // set up timer and interrupt

        let mut timer = Timer::tim2(dp.TIM2, Hertz(1), clocks);
        timer.listen(Event::TimeOut);
        
        //set up interrupts

        let exti = dp.EXTI;

        free(|cs| {
            TIMER_TIM2.borrow(cs).replace(Some(timer));
            EXTI.borrow(cs).replace(Some(exti));
            BUTTON.borrow(cs).replace(Some(board_btn));

        });

        let mut nvic = cp.NVIC;
            unsafe {
                nvic.set_priority(Interrupt::TIM2, 2);
                cortex_m::peripheral::NVIC::unmask(Interrupt::TIM2);
                nvic.set_priority(Interrupt::EXTI0, 1);
                cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI0);
            }
            cortex_m::peripheral::NVIC::unpend(Interrupt::TIM2);
            cortex_m::peripheral::NVIC::unpend(Interrupt::EXTI0);
                    
        // set the counter to some value, in this case 3 minutes
        // count down as long as the value > 0
        // set display to zero, blink the LED a few times
        // leave the LED on for three seconds
        
        loop {
           
            free(|cs| SET.borrow(cs).set(180));
            free(|cs| ELAPSED.borrow(cs).set(SET.borrow(cs).get()));

            while free(|cs| ELAPSED.borrow(cs).get()) > 0 {

                let mut buffer = ArrayString::<[u8; 64]>::new();

                let set = free(|cs| SET.borrow(cs).get()); 

                let elapsed = free(|cs| ELAPSED.borrow(cs).get()); 

                let e_hrs: u32 = elapsed / 3600;
                let e_mins: u32 = elapsed / 60;
                let e_secs: u32 = elapsed % 60;

                let s_hrs: u32 = set / 3600;
                let s_mins: u32 = set / 60;
                let s_secs: u32 = set % 60;
                
                format_time(&mut buffer, e_hrs as u8, e_mins as u8, e_secs as u8, s_hrs as u8, s_mins as u8, s_secs as u8);
                
                disp.write_str(buffer.as_str());
                
                delay.delay_ms(200_u16);

            }

            // display zeros
            
            let mut buffer = ArrayString::<[u8; 64]>::new();

            let zero: u8 = 0;

            let set = free(|cs| SET.borrow(cs).get()); 

            let s_hrs: u32 = set / 3600;
            let s_mins: u32 = set / 60;
            let s_secs: u32 = set % 60;

            format_time(&mut buffer, zero, zero, zero, s_hrs as u8, s_mins as u8, s_secs as u8);
                
            disp.write_str(buffer.as_str());
                
            // blink LED a few times, then leave it on

            for b in 0..11 { //odd number to keep the LED on after it's done blinking
                yellow.toggle();
                delay.delay_ms(100_u16);
            }

            delay.delay_ms(3000_u16);

            yellow.toggle();
        
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

        ELAPSED.borrow(cs).set(ELAPSED.borrow(cs).get() - 1);
        
    });
    
}


#[interrupt]

fn EXTI0() {

    // Enter critical section
    free(|cs| {
        // Obtain all Mutex protected resources
        if let (&mut Some(ref mut btn), &mut Some(ref mut exti)) = (
            BUTTON.borrow(cs).borrow_mut().deref_mut(),            
            EXTI.borrow(cs).borrow_mut().deref_mut()) {
         
            btn.clear_interrupt_pending_bit(exti);

            let timeset = SET.borrow(cs).get();

            ELAPSED.borrow(cs).replace(timeset);

        }

        
    });

}

fn format_time(buf: &mut ArrayString<[u8; 64]>, e_hrs: u8, e_mins: u8, e_secs: u8, s_hrs: u8, s_mins: u8, s_secs: u8) {
    fmt::write(buf, format_args!("    {:02}:{:02}:{:02}                                        {:02}:{:02}:{:02}    ",
    e_hrs, e_mins, e_secs, s_hrs, s_mins, s_secs)).unwrap();
}