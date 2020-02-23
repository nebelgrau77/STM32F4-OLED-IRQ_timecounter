Silent timer: counts back from a pre-set value (currently 3 minutes).

Counter is updated every second by TIM2 timer.

Elapsed time displayed on SSD1306 OLED in TerminalMode.

When the time is up, blinks LED three times and leaves the LED on for a second, then goes back to countdown. 

I tried adding a simple reset of the counter by pushing the button, but it is not working.

Future developments:

- time to be set with ADC + potentiometer
- clock start/stop/reset with a button
