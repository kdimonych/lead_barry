# lead_barry

# Overview

Lead Barry is the Battery Management System (BMS) for 12V lead acid batteries. The main goal of this project is to create remote controlled BMS.

The the firmware of the project is written in Rust for Raspberry Pi Pico target.

![General idea](./docs/idea.drawio.svg "General idea diagram")

# Features

- **🔒 Safe**: Written in safe Rust with comprehensive error handling
- **🚀 12 Volts Lead Acid BMS**: Designed to work with 12 volts lead acid batteries
- **IoT features**: Supports IoT features through onboard LoRa module and WiFi internet connection
- **LoRa**: Has LoRa module on board to provide the some IoT feaures
- **WiFi**: Capable of connecting to the local WiFi AP to provide remote control, or creating a WiFi AP to work in standalone mode
- **Web Interface**: Has implemented Web UI to configure, monitor, and control the device
- **🔧 WiFi configuration**: Configurable through WiFi network with Web UI
- **Controlled general purpouse DC outputs**: Has implemented two controlled DC outputs with selectable voltages 9/12V, one dedicated DC output with digitally controlled output voltage in the range 5 - 15V, and one controlled high-current bypass DC output with battery voltage on it
- **100Base Passive PoE**: Has a dedicated PoE injector socket to provide the power to Ethernet devices such as routers, etc. This socket has its own dedicated low-current DC/DC converter with adjustable output voltage in the range 5 - 15V to support various PoE-compatible devices
- **OLED Display and control buttons**: Has 128x64 monochrome OLED display and three buttons to provide manual control and basic configuration of the BMS

[More details](./docs/README.md "Details")

# Build and run

## Install Rust

Follow this instruction to [install Rust](https://rust-lang.org/tools/install/ "Instul Rust Instruction") on your system

## Install Rust toolchain + target

```bash
rustup target add thumbv6m-none-eabi
cargo install flip-link
```

## Build

```bash
cd firmware
cargo lead_barry_build
```

## Build and run

```bash
cd firmware
cargo lead_barry_run
```
