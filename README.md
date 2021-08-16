# weatherradio

`weatherradio` is a program that works in conjunction with [rtl_433](https://github.com/merbanan/rtl_433)
to capture local weather sensor data using a compatible Software
Defined Radio (SDR) device (e.g. RTL-SDR), and publish it to an mqtt broker.

# Compiling

`weatherradio` is written in [Rust](https://www.rust-lang.org/),
and can be built by installing rust via [rustup](https://rustup.rs/),
and then running `cargo build` from anywhere under the project root.

It also requires the `rtl_433` program to be available somewhere on
the system.

# Running

```
$ ls
weatherradio	rtl_433
$ weatherradio -r ./rtl_433
```
