> linux ready, win/mac in progress

## wled-cli
wled-cli is a lightweight, blazing-fast cli utility for wled devices, built with rust for sub-millisecond execution (<1ms) and minimal resource footprint (~880kb).

## usage
simply launch the interactive controller:
```
wled
```
### first launch
on first run with no devices configured, wled automatically opens the setup modal. device hardware capabilities and name are auto-detected from the wled firmware.

### controls
```
[arrows]             navigate devices, adjust brightness & temperature, pick colors
[space]               toggle power on / off
[m]                      cycle mode: rgb -> cct (cold/warm) -> on-off
[1] / [2] / [3]       cct quick presets: 2700k warm / 4200k neutral / 6500k cold
[enter]                apply selected color from palette
[tab]                    switch focus between panels
[a]                        add device (auto-detects hardware & name from ip)
[d] / [del]            delete selected device
[q] / [esc]            quit
```
  
## installation & build

### prebuilt binary (linux)
```bash
sudo curl -L "https://github.com/salfishlte/wled-cli/releases/latest/download/wled" -o /usr/local/bin/wled && sudo chmod +x /usr/local/bin/wled
```

### from source (cargo)
```bash
cargo install --git https://github.com/salfishlte/wled-cli
```
or build manually:
```bash
git clone https://github.com/salfishlte/wled-cli
cd wled-cli
cargo build --release
cp target/release/wled ~/.local/bin/
```

## credits
* **[wled](https://github.com/Aircoookie/WLED)** — wonderful project for controlling WS2812B LEDs <3
* **[rust](https://www.rust-lang.org/)** — core language for instant performance & tiny binary size
* **[ratatui](https://github.com/ratatui/ratatui)** — modern terminal user interface library
* **[ureq](https://github.com/algesten/ureq)** — lightweight synchronous HTTP client
