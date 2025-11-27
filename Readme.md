English | [Українська](README.uk.md)

<img src="images/photo.jpg" alt="Fan Controller Board" width="400"/>

# PWM Fan Controller Board

A control board for regulating fan speed using PWM (Pulse-Width Modulation).

## Overview

This board is designed to control up to four fans based on temperature readings from up to four NTC thermistors.  
It is intended, in particular, for use with hybrid inverters and includes an inverter fan emulation mode.

- Total output current: up to **5 A**, sufficient with a large margin for typical fans
- Temperature sensor type: **NTC 10 kΩ**
- Up to **4 fans** supported
- Up to **4 temperature sensors** supported
- **4 fan speed levels** depending on temperature
- **1 additional default speed** (used when temperature is below the first level)

## Fan–Sensor Mapping

Fan control logic is configurable in the menu:

- Any sensor can control any fan or group of fans
- All combinations are allowed

Examples:

- One sensor controls all 4 fans
- Two sensors:  
  - Sensor 1 controls fans 1–2  
  - Sensor 2 controls fans 3–4
- One sensor controls a single fan, another sensor controls the remaining three, etc.

The main menu displays for each fan/sensor:

- Temperature
- Fan speed (RPM)
- Current speed level (step)

## Hybrid Inverter Integration

The board is designed to work with a hybrid inverter and supports an **inverter fan emulation mode**.

- If the configuration allows the fans to be stopped (note: not all fans support full stop),
  the inverter will **not** generate a “fan stopped” error.
- Emulation is implemented by generating pulses at **300 Hz** on the signal (yellow) wire
  of the fan connector where the inverter’s original fan would normally be connected.

This corresponds to **9000 RPM** of the fan:

- 2 pulses per revolution  
- Formula: `300 / 2 * 60 = 9000`

### Fan Fault Handling

If the configuration requires a fan to rotate but it does not (for example):

- the connector is unplugged,
- the fan is blocked by a foreign object,
- the fan is defective,

then the inverter will detect an error and shut down until the cause is eliminated.  
This behaviour is intentional to prevent inverter overheating.

## Firmware / Development

### Flashing the firmware

```bash
cargo flash --chip stm32f411ceu6 --release
```

### Build and run with active terminal output

```bash
cargo run --release
```

### Build and detach from terminal

```bash
cargo embed --release
```

### Convert to HEX file

```bash
cargo-objcopy target/thumbv7em-none-eabihf/release/two --release -- -O ihex two.hex
```

### Disassembly

```bash
cargo objdump --bin two --release -- --disassemble --no-show-raw-insn --print-imm-hex
```

### Start OpenOCD

```bash
openocd -f interface/stlink.cfg -f target/stm32f4x.cfg
```

### Check program size

```bash
cargo size --bin fan_controller --release -- -A
```

## Memory Layout

### Program memory (Flash, base address: `0x08000000`)

- `.vector_table` – interrupt vector table, usually at the beginning of memory, containing addresses of interrupt handlers
- `.text` – section with program machine code (instructions)
- `.rodata` – section for constant, read-only data

### RAM (base address: `0x20000000`)

- `.data` – global and static variables initialized to specific values
- `.bss` – global and static variables initialized to zero
- `.uninit` – variables that are not initialized at program start

### Sections not included in the final binary / not critical for runtime

- `.gnu.sgstubs` – may contain special GNU-specific stubs
- `.ARM.attributes` – ARM-architecture-specific attributes
- `.comment` – comments or metadata
- `.defmt` – data for formatted output (e.g., for the `defmt` library)