# Super Mario Eclipse recomp bring-up

This workspace targets the local `GMSE04` Super Mario Eclipse 1.1.0 disc image.
The ISO, extracted DOL, disc files, generated Rust, and native executable are private build inputs and must not be distributed.

## One-time setup

Install rustup, then bootstrap the self-contained Windows toolchain:

```powershell
.\scripts\bootstrap-windows-toolchain.ps1
```

This keeps LLVM-MinGW and CMake under the parent workspace; no Visual Studio
installation or administrator access is required.

Prepare the private game files:

```powershell
.\scripts\prepare-eclipse.ps1 -IsoPath "D:\path\to\your-eclipse.iso"
```

This validates the disc header, extracts `eclipse/main.dol`, records hashes in `eclipse/manifest.json`, and builds the private `game/assets.bin` archive used by the virtual DVD drive.
The archive is streamed from disk and stays external to the executable.

Build the recomp:

```powershell
.\scripts\build-eclipse.ps1
```

The build uses Better Sunshine Engine's NTSC-U symbol map for retail SMS functions and automatically retains functions discovered only in Eclipse's patched DOL.

Run it:

```powershell
.\scripts\play-eclipse.ps1
```

## Controllers

Nintendo's `057e:0337` Wii U / Switch GameCube adapter and Mayflash's `0079:1843` adapter in Wii U mode are detected directly.
All four ports, analog sticks, analog triggers, digital L/R, Z, D-pad, and rumble are represented.

On Windows, the official adapter must expose a libusb-compatible interface.
If the input log says the adapter was found but could not be claimed, select WinUSB for the WUP-028 interface and reconnect it.
Ordinary controllers use SDL's mapping database.
The default layout follows physical GameCube positions: south=A, west=B, east=X, north=Y, triggers=L/R, and right bumper=Z.

Run the ten-second input diagnostic:

```powershell
.\scripts\input-doctor.ps1
```

## Mod workflow

The original Eclipse and Better Sunshine Engine files remain available through the prepared disc archive.

For instant asset iteration, place a replacement under `mods/files/` using its original disc path.
For example, `mods/files/map/scene.bin` overrides `map/scene.bin` without rebuilding the ISO or recomp.
Set `GCRECOMP_MOD_DIR` to use a different loose-file root.

Host-side Lua files placed directly in `mods/` load in filename order.
These are separate from PowerPC Better Sunshine Engine `.kxe` modules.

The SMS decomp and Better Sunshine Engine checkouts in the parent workspace are reference sources and symbol/header providers.
They are not copied into generated game code.

## Validated Eclipse milestone

The local 1.1.0 image currently produces:

- 13,496 discovered functions and 901,056 decoded instructions;
- 98.8% instruction translation coverage;
- a 111 MB generated Rust crate that passes `cargo check`;
- a linked native Windows debug host;
- a 473-file external DVD archive;
- successful initialization of Lua mods, DVD, the GameCube-adapter and SDL
  backends, Vulkan rendering, and host audio.

The smoke test reaches and returns from Eclipse's recompiled entry point, but
does not program a VI framebuffer. This is a bring-up executable, not playable
Sunshine yet.

## Honest compatibility status

This branch provides a reproducible Eclipse ingest path, stable symbols, direct controller support, and mod overlays.
It does not make the upstream experimental runtime a complete GameCube implementation.

The remaining boot-critical work is:

- continuous recompiled CPU execution integrated with the host event loop;
- GX command processing beyond the current framebuffer presentation path;
- DSP/audio behavior used by Sunshine and Better Sunshine Engine streaming;
- SI/PAD high-level emulation wired into generated calls;
- DVD async APIs, threads, interrupts, timing, memory cards, and OS services;
- Kuribo `.kxe` dynamic loading and relocations.

Until those services exist, a generated executable may compile but cannot be described as a playable PC port.
