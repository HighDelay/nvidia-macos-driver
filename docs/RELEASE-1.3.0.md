# 1401 and NVIDIA driver 1.3.0

**Something not working? [nullmothsystems.com/help](https://nullmothsystems.com/help) lists every known problem and exactly what to do.**

Changes
- **Send logs no longer freezes the Mac app.** Collecting runs in the background; the app keeps working (a few minutes). In 1.2.0 the whole app hung while it collected, so most people could not send logs at all.
- **Installing no longer sits at "Finding the OpenCore partition".** The password prompt now runs in its own process and always shows.
- **"1401: Remove NVIDIA driver" asks first.** In the boot picker it now waits for Y; any other key, or 60 seconds, changes nothing. Before, one stray Enter removed the driver ("it restarted and the driver was gone").
- While the driver is off for removal, the screen stays on.
- Windows (1401 1.3.0): Apple downloads finish on networks that cut every connection after 1 MB; a stick with an old macOS installer on it can be reused after you confirm; AMD PCs stuck at EXITBS:START get the memory settings that got a Ryzen 5 5500 / B550 to the installer; a failed start's log on the stick is read before the stick is erased.

Driver
- Kernel extensions and installer are the 1.2.0 ones (allow-list permission fix, remove flag, framebuffer step-aside).

Known issues, being fixed from your logs
- Two displays: changing one display's refresh rate or resolution can blank the other until restart.
- Refresh stuck at 60 Hz on some cards, glitches in the first minute after login, slow cursor on some laptops.
- Send logs from the Mac app right after any of these happens and post the report ID in the Discord.

Scope
macOS 15 Sequoia. Requires OpenCore.
