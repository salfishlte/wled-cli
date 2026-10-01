> linux ready, win/mac in progress

## wled-cli
wled-cli is a lightweight, blazing-fast TUI utility for controlling wled devices directly from your terminal without any heaps of complex code or complicated steps

it brings instant lighting control through an intuitive interactive menu, beyond the menu, you can control multiple devices in moment and adjust settings such as color, ссt, brightness, and the like.

## features

#### network scan
automatically searches for all **wled** devices on the local network

#### on/off
basic power switching

#### brightness
brightness changes in percent (0–100%)

#### color selection
you can select predefined colors from palettes or specify them using hex codes

#### cct control
adjustment of light temperature from warm to cool (0-255)

#### hardware setting (beta)
you can control the hardware, change the name of the device, the number of diodes, mA and other

## installation

to install the utility, simply run this one-line command in your terminal. it will automatically download the latest release and make it executable:
```
sudo curl -L "https://github.com/salfishlte/wled-pc/releases/latest/download/wled-pc" -o /usr/local/bin/wled && sudo chmod +x /usr/local/bin/wled
```

## credits

* **[wled](https://github.com/Aircoookie/WLED)** — wonderful project for controlling WS2812B LEDs <3
* **[python](https://www.python.org/)** — core language
* **[aiohttp](https://docs.aiohttp.org/)** — async HTTP requests & network scanning
* **[asyncio](https://docs.python.org/3/library/asyncio.html)** — concurrent I/O execution
