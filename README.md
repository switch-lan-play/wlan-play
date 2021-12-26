# wlan-play

A command line tool to play Nintendo Switch games locally remotely.

## Example config

```toml
# the wireless interface name, should be monitor mode
device = "wlan1mon"
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