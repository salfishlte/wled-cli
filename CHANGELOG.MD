## v0.1.3 // 10/30/26, 19:30
- features
  - added settings (network, power, about)
  - added signal status in main menu
- ui
  - some edits in main menu

## v0.1.2 // 10/19/26, 21:30
- core
  - complete rewrite from python to rust
  - sub-millisecond execution (<1ms) & lightweight binary (~940kb)
  - 0.0% idle cpu usage
- features
  - temporarily removed effects & settings

## v0.1.1 // 09/22/26, 23:51
- settings
  - added set_power_on_behavior, set_device_name, set_nightlight, set_udp_sync, set_led_output
  - added presets
  - added reboot
- caching
  - added wledctl_devices_cache.json (saving found devices)
- functions
  - added effects
  - added build.sh 
- optimization
  - optimization of finder.py and control.py
