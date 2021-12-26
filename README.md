# wlan-play

A command line tool to play Nintendo Switch games locally remotely.

## Usage

```shell
# 0. make sure wlan-play is installed
wlan-play --help
# 1. setup your wireless adapter.
# 1.1. add a monitor interface, replace the `wlan0` with your wireless adapter name.
iw dev wlan0 interface add mon0 type monitor
# 1.2. set the interface down
ip link set wlan0 down
# 1.3. set the monitor interface up
ip link set mon0 up
# 2. fill the config file with your server and password, then run `wlan-play`.
wlan-play -c <YOUR_CONFIG_FILE>
```

## Example config

```toml
# the wireless interface name, should be monitor mode
device = "mon0"
# "Host" or "Station"
mode = "Host"
# relay server
server = "127.0.0.1:19198"

[agent]
# Don't change this
platform = "Linux"
# the command to get a shell. you can just use "bash" if you want to use local shell
command = ["ssh", "rpi", "bash"]
# the [rhai](https://rhai.rs/) script to run after get shell.
# you must get root privilege after run this script.
after_connected = """
conn.send("exec sudo -s\\n");
conn.send("whoami\\n");
let id = conn.read_line();
debug("login as: " + id);
if id != "root\\n" {
    throw "not root";
}
"""
```