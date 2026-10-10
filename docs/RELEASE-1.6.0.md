# 1401 and NVIDIA driver 1.6.0

**Something not working? [nullmothsystems.com/help](https://nullmothsystems.com/help) lists every known problem and exactly what to do.**

## Driver / Mac
- **The driver no longer removes itself a few seconds after login.** On some laptops and boards, a change macOS makes to NVRAM never reaches the firmware, so an old "remove the driver" request from the boot picker came back at every start: the driver installed, macOS ran for a couple of seconds, restarted, and the driver was gone. Installing now hands that leftover request to OpenCore, which clears it before macOS starts.
- **Ryzen with your own OpenCore EFI: the driver starts instead of stopping at `RmInitAdapter failed! (0x25:0x40:1310)`.** If your config has Shaneee's "Fix PAT" patch on and Algrey's off, installing switches them (Algrey's is the one 1401 builds with, and the one that reached the desktop on the reporting machine).

## Windows (1401 1.6.0)
- **The Linux USB probe can now build your EFI.** If the Windows scan will not run or finish on your PC, the probe's report is enough for 1401 to build from.

## Ryzen 7000 / 9000 (AM5)
- If the stick stops with `EB.MM.AKM` / "Couldn't allocate", restart and pick the installer again 2–3 times before rebuilding — on AM5 this is often random from one start to the next.

## Scope
macOS 15 Sequoia. Requires OpenCore.
