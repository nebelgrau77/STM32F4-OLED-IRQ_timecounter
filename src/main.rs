//! Quiet timer (Work In Progress)
//! 
//! Platform: STM32F411 ("black pill" board)
//! 
//! Constantly update a counter and display it as elapsed time.
//! 
//! Uses an OLED SSD1306 display with I2C interface, an LED and a button.
//! 
//! It counts down from 180 seconds, then blinks the LED a few times, then goes back to countdown.
//! 
//! Pressing the button resets the counter back to 180 seconds.
//! 
//! Both elapsed time and set counter time are displayed in TerminalMode.
//! 
//! Time update is controlled by TIM2 timer, firing every second. 
//! Display is updated every 200 ms with less precise SysClock.
//! 
//! Further developments:
//! 
//! - use ADC to set the counter time
//! - use button to stop/start/reset the counter
//! 
//! Connections:
//! 
//! I2C:
//! SDA -> PB9
//! SCL -> PB8
//!
//! LED: PA1
//! 
//! BUTTON: built-in button on PA0
//! //! 
//! 
//! Best results when using `--release`.

#![no_std]
#![no_main]

// import all the necessary crates and components

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate stm32f4xx_hal as hal;
extern crate stm32f4;
extern crate panic_halt;

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

use crate::hal::{
    prelude::*,
    gpio::{gpioa::PA0, Edge, ExtiPin, Input, PullUp},
    i2c::I2c,
    stm32,
    timer::{Timer, Event},
    delay::Delay,
    time::Hertz,
    stm32::{Interrupt,EXTI},
        
};


// create two globally accessible values for set and elapsed time
static SET: Mutex<Cell<u32>> = Mutex::new(Cell::new(0u32));
static ELAPSED: Mutex<Cell<u32>> = Mutex::new(Cell::new(0u32));

// globally accessible interrupts and peripherals: timer, external interrupt and button
static TIMER_TIM2: Mutex<RefCell<Option<Timer<stm32::TIM2>>>> = Mutex::new(RefCell::new(None));
static EXTI: Mutex<RefCell<Option<EXTI>>> = Mutex::new(RefCell::new(None));
static BUTTON: Mutex<RefCell<Option<PA0<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    if let (Some(mut dp), Some(cp)) = (
        stm32::Peripherals::take(),
        cortex_m::peripheral::Peripherals::take(),
    ) {
        
        // necessary to enable this for the external interrupt to work
        dp.RCC.apb2enr.write(|w| w.syscfgen().enabled()); 

        // set up clocks
        let rcc = dp.RCC.constrain();
        let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

        // Set up I2C - SCL is PB8 and SDA is PB9; they are set to Alternate Function 4, open drain
        let gpiob = dp.GPIOB.split();
        let scl = gpiob.pb8.into_alternate_af4().set_open_drain();
        let sda = gpiob.pb9.into_alternate_af4().set_open_drain();
        let i2c = I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks);

        //set up LED on pin PA1
        let gpioa = dp.GPIOA.split();
        let mut yellow = gpioa.pa1.into_push_pull_output();
        
        //set up the on-board button on PA0
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

        // set up timer and external interrupt
        let mut timer = Timer::tim2(dp.TIM2, Hertz(1), clocks);
        timer.listen(Event::TimeOut);

        let exti = dp.EXTI;

        free(|cs| {
            TIMER_TIM2.borrow(cs).replace(Some(timer));
            EXTI.borrow(cs).replace(Some(exti));
            BUTTON.borrow(cs).replace(Some(board_btn));

        });


        let mut nvic = cp.NVIC;
            unsafe {
                nvic.set_priority(Interrupt::TIM2, 1);
                cortex_m::peripheral::NVIC::unmask(Interrupt::TIM2);
                nvic.set_priority(Interrupt::EXTI0, 2);
                cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI0);
            }
            cortex_m::peripheral::NVIC::unpend(Interrupt::TIM2);
            cortex_m::peripheral::NVIC::unpend(Interrupt::EXTI0);
                    
        // set the counter to some value, in this case 3 minutes
        // count down as long as the value > 0
        
        loop {
           
            free(|cs| SET.borrow(cs).set(180));
            free(|cs| ELAPSED.borrow(cs).set(SET.borrow(cs).get()));

            while free(|cs| ELAPSED.borrow(cs).get()) > 0 {

                // create an empty buffer for the display
                let mut buffer = ArrayString::<[u8; 64]>::new();

                // get the values from the global variables
                let elapsed = free(|cs| ELAPSED.borrow(cs).get()); 
                let set = free(|cs| SET.borrow(cs).get()); 

                // convert the seconds to hh:mm:ss format

                let (e_hrs, e_mins, e_secs) = time_digits(elapsed);
                let (s_hrs, s_mins, s_secs) = time_digits(set);

                // format the current time values and write them on the display
                
                format_time(&mut buffer, e_hrs, e_mins, e_secs, s_hrs, s_mins, s_secs);
                
                disp.write_str(buffer.as_str());
                
                delay.delay_ms(200_u16);

            }

            // display zeros when the time is up
            
            let mut buffer = ArrayString::<[u8; 64]>::new();

            let zero: u8 = 0;

            let set = free(|cs| SET.borrow(cs).get()); 

            let (s_hrs, s_mins, s_secs) = time_digits(set);

            format_time(&mut buffer, zero, zero, zero, s_hrs, s_mins, s_secs);
                
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

    // enter critical section

    free(|cs| {
        stm32::NVIC::unpend(Interrupt::TIM2);
        if let Some(ref mut tim2) = TIMER_TIM2.borrow(cs).borrow_mut().deref_mut() {
            tim2.clear_interrupt(Event::TimeOut);
        }

        // decrease the ELAPSED value by 1 second

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

            // set the ELAPSED value back to the SET value

            let timeset = SET.borrow(cs).get();

            ELAPSED.borrow(cs).replace(timeset);

        }
        
    });

}

// helper function for the display
// in TerminalMode there are 64 characters in 4 lines (128x32 display, 8x8 characters)
// to avoid the content being moved accross the display with every update
// the buffer content must always be 64 characters long

fn format_time(buf: &mut ArrayString<[u8; 64]>, e_hrs: u8, e_mins: u8, e_secs: u8, s_hrs: u8, s_mins: u8, s_secs: u8) {
    fmt::write(buf, format_args!("    {:02}:{:02}:{:02}                                        {:02}:{:02}:{:02}    ",
    e_hrs, e_mins, e_secs, s_hrs, s_mins, s_secs)).unwrap();
}


// helper function to convert seconds to hours, minutes and seconds    

fn time_digits(time: u32) -> (u8, u8, u8) {
    
    let hours = time / 3600;
    let minutes = time / 60;
    let seconds = time % 60;

    (hours as u8, minutes as u8, seconds as u8)
}

