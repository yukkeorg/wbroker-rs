# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

WBroker-rs is a Rust-based temperature and humidity monitoring system designed for Raspberry Pi Zero 2 W. It interfaces with BME280 environmental sensors and displays readings on SO1602A OLED character displays. The system calculates and shows temperature-humidity index (THI) along with current date/time.

## Architecture

The project consists of two main components:

1. **Main Application** (`src/main.rs`): Core loop that reads sensor data, calculates THI, and updates the display every 200ms
2. **Peripheral Library** (`peripheral/`): Hardware abstraction layer containing:
   - `bme280.rs`: BME280 sensor driver for temperature/humidity/pressure readings
   - `so1602a.rs`: SO1602A OLED display driver with custom character support

The peripheral library uses `rppal` for Raspberry Pi GPIO/I2C communication.

## Build Commands

The project uses cross-compilation for ARM deployment:

```bash
# Build release package for Raspberry Pi Zero 2 W
make

# Clean build artifacts
make clean

# Cross-compile binary only
cross build --target armv7-unknown-linux-gnueabihf --release
```

Target architecture is `armv7-unknown-linux-gnueabihf` for Raspberry Pi Zero 2 W compatibility.

## Deployment

The build process creates `dist/wbroker-rs.tar.gz` containing:
- Cross-compiled binary
- systemd service file (`wbroker-rs.service`)
- Installation Makefile

Installation on target device:
```bash
# Extract and install
tar -xzf wbroker-rs.tar.gz
cd wbroker-rs
make install

# Start service
sudo systemctl start wbroker-rs
```

## Development Notes

- Uses `cross` for cross-compilation (requires Docker)
- Release profile optimized for embedded deployment (LTO, size optimization)
- No standard testing framework configured
- Hardware-dependent code requires actual Raspberry Pi for testing