# Mods

GCRecomp supports two local mod layers.

Put host-side Lua mods directly in this directory.
They load in filename order, so names such as `10-practice.lua` make composition deterministic.

Put loose replacements for files from the GameCube disc under `mods/files/`.
The directory layout must match the disc path.
For example, `mods/files/map/scene.bin` overrides `map/scene.bin` without rebuilding the ISO or the recomp.

Existing Better Sunshine Engine modules inside the prepared Eclipse disc remain part of `game/assets.bin`.
PowerPC `.kxe` modules still depend on the recomp runtime implementing every GameCube SDK service they use.
Native host hooks are intentionally separate from `.kxe` modules.
