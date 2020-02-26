Silent timer: counts back from a pre-set value.

Counter is updated every second by TIM2 timer.

Time to count down from is set in 30-second intervals with a potentiometer/ADC.

Elapsed time displayed on SSD1306 OLED in TerminalMode.

When the time is up, blinks LED three times and leaves the LED on for three second, then goes back to countdown. 

Pressing the on-board button resets the timer back to the set time. 

Future developments:

- improve the ADC reading (currently the values are constantly oscillating, have to be averaged somehow)
- time to be set with ADC + potentiometer