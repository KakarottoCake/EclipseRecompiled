# Installation and local build

Eclipse Recompiled is pre-alpha developer software. There are no downloadable
game builds or supported releases.

The complete, current Windows setup is in
[README.md](README.md#building-locally). In short:

```powershell
git clone https://github.com/KakarottoCake/EclipseRecompiled.git
cd EclipseRecompiled
.\scripts\bootstrap-windows-toolchain.ps1
git clone https://github.com/DotKuribo/BetterSunshineEngine.git ..\third_party\BetterSunshineEngine
.\scripts\prepare-eclipse.ps1 -IsoPath "D:\path\to\your-eclipse.iso"
.\scripts\build-eclipse.ps1
.\scripts\play-eclipse.ps1
```

You must supply a legally obtained Super Mario Eclipse 1.1.0 `GMSE04` image.
The repository does not contain the game or generated game code.

The resulting program is a runtime bring-up executable and is not playable.
See [What is still missing?](README.md#what-is-still-missing) before building.
