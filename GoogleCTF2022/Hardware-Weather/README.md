<style>
    p{
        text-align: justify
    }
</style>
# GoogleCTF 2022 - Hardware - Weather writeup
This challenge is what I consider a good learning experience, 
and a perfect model for how hardware challenges should be like. 
It was really interesting not only in terms of the problems you needed to
solve to get the flag, but also in terms of the additional
research you needed to do to get a clear picture of how the system works
exactly. I would definitely spend hours solving challenges like this
one again.

And don't expect to just sit there and read; reading is lame,
this writeup is also a guided exercice,
and I would recommend taking a pen, a paper, and a cup of coffee
to enjoy the full experience.

## Challenge description
> Our DYI Weather Station is fully secure! No, really! Why are you
> laughing?! OK, to prove it we're going to put a flag in the internal
> ROM, give you the source code, datasheet, and network access to the
> interface.

## Attachements
[firmware.c] \
[Device Datasheet Snippets.pdf]

## Remote connection
If the servers of the challenge are still running, then you can connect to them at `weather.2022.ctfcompetition.com 1337`.

Otherwise you would need to compile them from 
[sources](https://github.com/google/google-ctf/tree/master/2022/hardware-weather/challenge), 
I haven't tried doing it, but it should be simple since they provide 
a Dockerfile. Keep in mind that you aren't allowed to look at the 
sources before solving the challenge.

## Prerequesite knowledge
The line separating *Prerequisite knowledge* and *Stuff to search for* 
is arbitrary, and in fact it can be argued that they are the same thing,
but in this writeup I will be drawing that line based on what I already knew
before this challenge.

### SFRs — Special Function Registers
For microcontrollers, it is very common to have special registers 
that exhibit special behaviour when read or written to, or control the
microcontroller itself. These registers are (usually) accessed through RAM
addresses, and so the same instructions to modify any memory byte applies
to them as well.

### I2C
I2C is a protocol that allows multiple devices to communicate with each
other. The details of exactly how the protocol works are not important
as they are abstracted away by the microcontroller, all that is needed
to know is that each device connected to the I2C bus has a port and
can transfer any size data to the microcontroller when requested.

## Initial experiments
If we try to open a remote connection with the program through a utility
such as `ncat` with `ncat weather.2022.ctfcompetition.com 1337`, we get
what seems like a command prompt `? `. Trying some commands like `ls`,
`echo`, `cat`... achieves nothing, so it definitely isn't a shell, `help`
doesn't help either ;)

At his point no more blinf testing can be done, so it's time to read
the datasheet and the source code.

## Walking through the datasheet
The datasheet provided contains a good overview of the whole
system, and even more details about how to control the different devices.

### Overview
![Circuit diagram](res/circuit.png)

From the circuit diagram, we can see that the system is composed of:
* A microcontroller(CTF-8051)
* EEPROM(CTF-55930D)
* Multiple sensors
* I2C bus(SDA, SCL)
* Serial IO(Stx, Srx)

All the sensors and the EEPROM are connected with the microcontroller
through the I2C bus.

By googling [8051], we can find that there is a processor with that name,
this could be useful if, say *cough cough* for a reason, we needed 
to reprogram the device *cough cough*, and just in general to have a
better understanding of the components in the system.

Further down we have a table giving the I2C ports
of the sensors.

![Sensors-Ports table](res/sensors.png)

### Devices Interface
The datasheet also describes in detail how to interact with the different devices connected to the microcontroller. In short, it specifies:
* Serial IO, through SFRs.
* I2C Controller, through SFRs.
* FlagROM, through SFRs.
* Sensors, through I2C.
* EEPROM, through I2C.
* Pinging I2C devices.

Also, quoting from the datasheet:
> In a typical application CTF-55930B serves as firmware storage for 
> CTF-8051 microcontroller via the SPI(PMEM) bus.

So the EEPROM stores the program code, in addition, the datasheet describes
the protocol to clear bits from the EEPROM(setting bits is impossible
without physical access to the device), effectivly explaining how 
to reprogram the EEPROM with some constraints. This definitely could be used
to exploit the system.

## Walking through the source code
Right at the beginning of `firmware.c`, we can see a bunch of declarations using a special syntax:
```c
// Secret ROM controller.
__sfr __at(0xee) FLAGROM_ADDR;
__sfr __at(0xef) FLAGROM_DATA;

// Serial controller.
__sfr __at(0xf2) SERIAL_OUT_DATA;
__sfr __at(0xf3) SERIAL_OUT_READY;
__sfr __at(0xfa) SERIAL_IN_DATA;
__sfr __at(0xfb) SERIAL_IN_READY;

// I2C DMA controller.
__sfr __at(0xe1) I2C_STATUS;
__sfr __at(0xe2) I2C_BUFFER_XRAM_LOW;
__sfr __at(0xe3) I2C_BUFFER_XRAM_HIGH;
__sfr __at(0xe4) I2C_BUFFER_SIZE;
__sfr __at(0xe6) I2C_ADDRESS;  // 7-bit address
__sfr __at(0xe7) I2C_READ_WRITE;

// Power controller.
__sfr __at(0xff) POWEROFF;
__sfr __at(0xfe) POWERSAVE;
```
It contains a declaration for all the SFR ports described in the datasheet,
in addition to `POWEROFF` and `POWERSAVE`, which were left undocumented.

Anyway, as the law of reverse engineering dictates, we must start 
reading the source code from the main function, I would recommand taking
a look at the function, but tl;dr: it continously checks for user commands received via the serial input and handles them.

The two valid commands are:

`r P L`: Read L bytes from I2C device at port P.

`w P L D[L]`: Write L bytes from D to I2C device at port P.

Remember talking about reprogramming the EEPROM through the I2C interface
well, it seems easy now that we have the `w` command, no?

Well, there are two problems.

### Port verification
The original function of the entire system is to report different 
atmospherical data, and so the developpers limited reading and writing to
I2C to sensor ports through the function `bool is_port_allowed(char*)`.

As a result we are limited to the ports:
```c
const char *ALLOWED_I2C[] = {
  "101",  // Thermometers (4x).
  "108",  // Atmospheric pressure sensor.
  "110",  // Light sensor A.
  "111",  // Light sensor B.
  "119",  // Humidity sensor.
  NULL
};
```

The declaration of that function goes as follow:
```c
bool is_port_allowed(const char *port) {
  for(const char **allowed = ALLOWED_I2C; *allowed; allowed++) {
    const char *pa = *allowed;
    const char *pb = port;
    bool allowed = true;
    while (*pa && *pb) {
      if (*pa++ != *pb++) {
        allowed = false;
        break;
      }
    }
    if (allowed && *pa == '\0') {
      return true;
    }
  }
  return false;
}
```
What we would like to have is the function reaching `return true` for any
arbitrary port, the only way to reach it is by having `allowed` still set
to `true` and reaching the end of the string from `ALLOWED_I2C`. 
`while(*pa&& *pb)` will loop until either `*pa == 0` or `*pb == 0`, so 
if the `pa` is the string that terminates first, we can have the right part
of the condition `allowed && *pa == '\0'` guarentedd to be true. As for
`allowed`, it is only set to `false` when there `*pa != *pb` inside the loop, the same loop that breaks if any of the strings reaches the end.

If it didn't become obvious already, this function can be exploited by
having `port` start with a valid port sequence(example `101`) followed by
any arbitrary number to make it overflow to the desired port value when
converted to a `uint8_t`.



[firmware.c]: res/firmware.c
[Device Datasheet Snippets.pdf]: res/Device%20Datasheet%20Snippets.pdf

[8051]: https://en.wikipedia.org/wiki/Intel_8051
