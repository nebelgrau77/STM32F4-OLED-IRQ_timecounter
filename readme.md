WILL NOT COMPILE

Cannot read ADC with the current code. 

Error: 

error[E0277]: the trait bound `&mut hal::gpio::gpioa::PA3<hal::gpio::Input<hal::gpio::Analog>>: hal::embedded_hal::adc::Channel<hal::stm32::ADC1>` is not satisfied
   --> src/main.rs:307:38
    |
307 |             let sample = adc.convert(&analog, SampleTime::Cycles_480);
    |                                      ^^^^^^^ the trait `hal::embedded_hal::adc::Channel<hal::stm32::ADC1>` is not implemented for `&mut hal::gpio::gpioa::PA3<hal::gpio::Input<hal::gpio::Analog>>`
    |
    = help: the following implementations were found:
              <hal::gpio::gpioa::PA3<hal::gpio::Analog> as hal::embedded_hal::adc::Channel<hal::stm32::ADC1>>



Silent timer: counts back from a pre-set value (currently 3 minutes).

Counter is updated every second by TIM2 timer.

Elapsed time displayed on SSD1306 OLED in TerminalMode.

When the time is up, blinks LED three times and leaves the LED on for three second, then goes back to countdown. 

Pressing the on-board button resets the timer back to 180 seconds. 

Future developments:

- time to be set with ADC + potentiometer
- clock start/stop/reset with a button
