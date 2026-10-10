# 1401 and NVIDIA driver 1.10.0

**Something not working? [nullmothsystems.com/help](https://nullmothsystems.com/help) lists every known problem and exactly what to do.**

## Windows (1401 1.10.0)
- Ryzen 7000, 8000 and 9000 PCs (AM5 and Zen 4/5 laptops) stuck at the prohibited sign / `EB.MM.AKM`: the first build now uses the memory setting these CPUs boot with (DevirtualiseMmio on). Build the stick again with 1.10 — rebooting an old stick changes nothing.
- No-name sticks that Windows doesn't show after the erase are used through the drive letter Windows' diskpart gives them.
- The stick gets the driver package that matches the app, even when older versions are in the same folder, and a driver download that stops early is fetched again.

## Driver / Mac
- **Remove driver** works when the driver was installed from the stick or by hand (no install record): its settings come out of the OpenCore config this Mac started from and its files are removed.

## Scope
macOS 15 Sequoia. Requires OpenCore.
