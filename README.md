# Eclipse Recompiled

> **Experimental bring-up project — not a playable PC port yet.**

[![Status: pre-alpha](https://img.shields.io/badge/status-pre--alpha-orange)](#current-status)
[![CI](https://github.com/KakarottoCake/EclipseRecompiled/actions/workflows/ci.yml/badge.svg)](https://github.com/KakarottoCake/EclipseRecompiled/actions/workflows/ci.yml)
[![License: CC0-1.0](https://img.shields.io/badge/license-CC0--1.0-lightgrey.svg)](LICENSE)

Eclipse Recompiled is an experiment in turning **Super Mario Eclipse 1.1.0**
into a native Windows program using
[GameCubeRecompiled](https://github.com/KaiserGranatapfel/GameCubeRecompiled).
The long-term goal is a version that feels natural on a PC, recognizes a real
GameCube controller adapter, supports ordinary modern controllers, and is easy
to modify.

The project can already read a legally obtained Eclipse disc image, translate
its main program into Rust, compile a Windows executable, load its files, and
start the host-side runtime. It **cannot reach menus or gameplay** because
important parts of the underlying GameCube runtime are still unfinished.

## Please read this first

- This repository contains **no game, ROM, ISO, DOL, Nintendo assets, or
  recompiled game binary**.
- You must provide your own legally obtained `GMSE04` Super Mario Eclipse 1.1.0
  image.
- Generated game code and extracted assets remain local and are ignored by Git.
- A successful build does not mean the game is playable.
- This project is unofficial and is not affiliated with Nintendo, the Super
  Mario Eclipse team, or the upstream projects named below.

## What does “recompilation” mean?

A GameCube game contains PowerPC machine instructions. A static recompiler
translates those instructions ahead of time into code a modern PC can run.

That translation is only half the job. The original game also expects the
GameCube’s graphics chip, audio processor, controllers, disc drive, operating
system services, timing, and memory behavior. The PC runtime must provide
working replacements for all of them.

An everyday analogy:

- The recompiler translates the game’s **language**.
- The runtime recreates the **console the game expects to live in**.

Eclipse’s language is now mostly translated. The recreated console is not
complete enough for Sunshine to run.

## Current status

| Area | State | What that means |
| --- | --- | --- |
| Eclipse disc validation | Working | Recognizes the `GMSE04` 1.1.0 image and records hashes |
| DOL extraction | Working | Extracts the game’s main PowerPC executable locally |
| Static translation | Working, incomplete | Generates about 111 MB of Rust from the patched Eclipse DOL |
| Native Windows build | Working | The generated crate checks and a debug host links successfully |
| Startup | Partial | Eclipse’s recompiled entry point runs and returns |
| Disc filesystem | Working | 473 files are available through an external, compressed archive |
| Host Lua mods | Working | Local `.lua` files load in filename order |
| Loose asset overrides | Working | Files under `mods/files/` can replace disc files |
| GameCube adapter detection | Implemented | Nintendo and Mayflash adapters are handled directly through USB |
| Modern controllers | Implemented on host | SDL maps Xbox, PlayStation, Switch, and similar controllers |
| In-game controller input | Not finished | The host input must still be connected to Sunshine’s PAD/SI calls |
| Graphics | Not game-ready | A Vulkan host renderer starts, but Sunshine does not produce a usable framebuffer |
| Audio | Not game-ready | Host audio starts, but GameCube DSP behavior is incomplete |
| Menus and gameplay | Not working | The game does not boot far enough yet |
| Better Sunshine/Kuribo modules | Not working | Native `.kxe` module loading and relocation are not implemented |

### Last validated local milestone

Using Super Mario Eclipse 1.1.0:

- 13,496 functions discovered;
- 901,056 PowerPC instructions decoded;
- 98.8% instruction translation coverage;
- 58 core and runtime tests passing;
- the full generated crate passes compilation checks;
- a 353 MB debug host links;
- Vulkan rendering, host audio, Lua, DVD, SDL, and the GameCube adapter backend
  initialize during a smoke test.

The percentage above measures whether the recompiler emitted code for an
instruction. It does **not** prove that every translated instruction behaves
correctly.

## What is still missing?

Most remaining blockers are capabilities that
[GameCubeRecompiled](https://github.com/KaiserGranatapfel/GameCubeRecompiled)
does not yet provide completely. Some work will be useful to every GameCube
recomp project; other work is specific to Sunshine and Eclipse.

### 1. A continuous game execution loop

The host currently calls Eclipse’s entry point during startup. A real game must
keep executing while the window handles frames, input, audio, interrupts, and
timers. These systems need a reliable scheduler instead of a one-time call and
watchdog.

### 2. GameCube operating-system behavior

Sunshine relies on Nintendo’s GameCube OS behavior. The runtime still needs
more complete implementations of:

- threads and context switching;
- alarms and timers;
- interrupts;
- message queues and synchronization;
- cache and memory behavior;
- exception handling.

Small inaccuracies here can make the game wait forever or take the wrong code
path.

### 3. GX graphics support

The GameCube’s GX graphics system must be translated into modern GPU commands.
The host can create a Vulkan window and present a framebuffer, but Sunshine
uses substantially more GX state, vertex formats, textures, copy operations,
and synchronization than the runtime currently handles.

This is the largest visible blocker: until GX is sufficiently complete, there
is no real game image to display.

### 4. DSP and streaming audio

Starting a PC audio device is not the same as reproducing the GameCube audio
processor. Sunshine and Better Sunshine Engine expect DSP tasks, mixing,
streaming, timing, and callbacks that are still missing.

### 5. Connecting controllers to the game

The host-side controller layer is implemented:

- Nintendo `057e:0337` adapters;
- Mayflash `0079:1843` adapters in Wii U mode;
- four GameCube controller ports;
- analog sticks and triggers;
- rumble;
- sensible modern-controller face-button mapping.

Sunshine cannot use that data until the runtime implements and dispatches the
GameCube `PAD` and `SI` APIs that the translated game calls.

### 6. Disc, save, and asynchronous I/O behavior

Basic file lookup and reading work. The game also expects asynchronous DVD
requests, callbacks, priorities, timing, cancellation, error states, and memory
card services. These need to behave closely enough to the original hardware.

### 7. Recompiler correctness

The complete generated Eclipse crate compiles, but deeper boot testing will
almost certainly reveal translated instructions, indirect branches, register
behavior, or function boundaries that need correction. “Compiles” and
“matches the original console” are very different standards.

### 8. Better Sunshine Engine and Kuribo support

Eclipse uses
[Better Sunshine Engine](https://github.com/DotKuribo/BetterSunshineEngine).
Its Kuribo `.kxe` modules need a loader that understands module metadata,
relocations, imports, exports, and lifecycle callbacks. The current host Lua
layer is useful for PC-side experiments, but it is not a replacement for
Kuribo.

### 9. Eclipse-specific testing and fixes

Once ordinary Super Mario Sunshine can boot through the runtime, Eclipse’s
custom code, stages, assets, and engine changes must be tested individually.
Game-specific shims may still be required.

## When will this be considered ready?

The project should not be called finished until it can:

- boot reliably to the Eclipse title screen;
- enter stages and sustain gameplay;
- render correctly across common GPUs;
- play music and sound without major timing problems;
- use a real GameCube adapter and common modern controllers;
- save and load safely;
- load the Better Sunshine/Kuribo modules Eclipse depends on;
- run without requiring copyrighted files in the repository or release;
- provide a repeatable build process from a user-owned image.

## Controller goals

The intended default behavior is:

- a real GameCube adapter should work with minimal setup;
- all four adapter ports should be visible;
- analog L/R and digital trigger clicks should both work;
- rumble should work;
- modern controllers should follow the physical GameCube layout:
  south = A, west = B, east = X, north = Y, triggers = L/R, right bumper = Z;
- advanced remapping can be added later without making the default setup
  confusing.

On Windows, an official adapter normally needs its interface exposed through
WinUSB. The input log explains this when an adapter is visible but cannot be
claimed.

## Modding goals

Eclipse is already built on
[Better Sunshine Engine](https://github.com/DotKuribo/BetterSunshineEngine)
and the [Super Mario Sunshine decompilation](https://github.com/doldecomp/sms).
This project aims to preserve that mod-friendly spirit.

Two early workflows exist:

1. **Loose file overrides**

   Put a replacement at `mods/files/<original-disc-path>`. It takes precedence
   over the archived disc file without rebuilding the ISO.

2. **Host Lua scripts**

   Put `.lua` files directly in `mods/`. They load in filename order when the
   host starts.

Kuribo `.kxe` support remains a separate, unfinished requirement.

## Building locally

The tested setup is Windows. First builds are large and may take several
minutes.

### Requirements

- Windows 10 or 11;
- [rustup](https://rustup.rs/);
- Python with `pip`;
- a legally obtained Super Mario Eclipse 1.1.0 `GMSE04` image;
- several gigabytes of free disk space.

### 1. Clone and bootstrap

```powershell
git clone https://github.com/KakarottoCake/EclipseRecompiled.git
cd EclipseRecompiled
.\scripts\bootstrap-windows-toolchain.ps1
git clone https://github.com/DotKuribo/BetterSunshineEngine.git ..\third_party\BetterSunshineEngine
```

The bootstrap script installs portable LLVM-MinGW and CMake under the parent
workspace. It does not require Visual Studio or administrator access.

### 2. Prepare your private image

```powershell
.\scripts\prepare-eclipse.ps1 -IsoPath "D:\path\to\your-eclipse.iso"
```

This creates ignored local files:

- `eclipse/main.dol`;
- `eclipse/manifest.json`;
- `game/assets.bin`;
- generated memory-image data.

Do not commit or distribute them.

### 3. Generate and build

```powershell
.\scripts\build-eclipse.ps1
```

The build uses Better Sunshine Engine’s NTSC-U symbol map when its repository
is checked out beside this one at:

```text
../third_party/BetterSunshineEngine/maps/us.map
```

You can also pass a map explicitly:

```powershell
.\scripts\build-eclipse.ps1 -SymbolMap "D:\path\to\us.map"
```

### 4. Run the experimental host

```powershell
.\scripts\play-eclipse.ps1
```

Expect a bring-up window, not a playable game.

### 5. Inspect controllers

```powershell
.\scripts\input-doctor.ps1
```

The diagnostic watches for ten seconds and prints connected controllers and
live input.

## Repository layout

```text
gcrecomp-core/       PowerPC decoding, translation, memory, and SDK services
gcrecomp-runtime/    Graphics, audio, controller, and host runtime
gcrecomp-cli/        Analyze, prepare, and recompile commands
gcrecomp-lua/        Host-side Lua and disc archive helpers
gcrecomp-ui/         Host configuration UI
game/                Native host application
recompiled/          Placeholder for generated private game code
mods/                Host Lua examples and ignored loose asset overrides
scripts/             Windows bootstrap, build, launch, and diagnostics
docs/ECLIPSE_PORT.md Technical bring-up notes and validated measurements
```

## How to help

Contributions are welcome, but this is currently a low-level runtime project,
not a content-mod project. The most useful areas are:

- GX command and shader behavior;
- OS threads, interrupts, timers, and queues;
- PAD/SI high-level emulation;
- DSP and streaming audio;
- DVD asynchronous APIs and memory cards;
- PowerPC translation correctness tests;
- Kuribo `.kxe` loading;
- small, reproducible Sunshine boot tests.

Please do not open issues asking for game downloads or attach copyrighted game
files. Reports should contain logs, addresses, hashes where appropriate, and
the smallest reproducible technical case.

## Frequently asked questions

### Is it playable?

No. It compiles and starts, but does not reach menus or gameplay.

### Does the repository include Super Mario Eclipse?

No. It contains only source code and tooling.

### Why not just use Dolphin?

Dolphin is the practical way to play Eclipse today. This project explores a
native static-recompilation and modding path. It is research and development,
not a Dolphin replacement at its current stage.

### Does a compiling executable mean the recompiler is finished?

No. Compilation proves that Rust accepted the generated source. It does not
prove that the result reproduces GameCube hardware behavior.

### Will upstream GameCubeRecompiled improvements help?

Yes. General GX, DSP, OS, DVD, and input improvements should benefit this
project. Sunshine/Eclipse integration and Kuribo support will still require
game-specific work.

## Upstream projects and attribution

This repository is a development fork of
[KaiserGranatapfel/GameCubeRecompiled](https://github.com/KaiserGranatapfel/GameCubeRecompiled)
and preserves its Git history and CC0-1.0 license.

It uses or references public information from:

- [DotKuribo/BetterSunshineEngine](https://github.com/DotKuribo/BetterSunshineEngine);
- [doldecomp/sms](https://github.com/doldecomp/sms);
- [mstorsjo/llvm-mingw](https://github.com/mstorsjo/llvm-mingw).

Those projects are not bundled here and retain their own authorship and
licenses.

## Legal notice

This repository is intended for interoperability, research, preservation, and
development using legally obtained game data. It does not grant rights to
Nintendo, Super Mario, Super Mario Sunshine, Super Mario Eclipse, or any other
third-party material. Do not distribute disc images, extracted assets,
generated game code, or compiled game binaries.

See [LICENSE](LICENSE) and [EULA.md](EULA.md) for this repository’s existing
terms.
