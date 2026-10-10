#!/usr/bin/env python3
"""The driver needs Lilu + AMFIPass on in OpenCore. 1.9.0 logs (10-10, Ryzen 9 9900X, same SIP bits as a working
machine): AMFIPass was in the config but off, WindowServer refused the plugin ("mapping process is a platform binary,
but mapped file is not") and the screen stayed black with a cursor. Runs the setup's real step on fake configs."""
import os
import pathlib
import plistlib
import subprocess
import tempfile
import unittest

SETUP = pathlib.Path(__file__).resolve().parents[1] / "app/Resources/nullmoth-setup.sh"
START = "# AMFIPass (a Lilu plugin) is what lets WindowServer"
END = "# The macOS installer boots with a small GPU BAR"


def kext(name, enabled=True):
    return {"Arch": "Any", "BundlePath": name, "Enabled": enabled, "ExecutablePath": f"Contents/MacOS/{name[:-5]}",
            "PlistPath": "Contents/Info.plist", "Comment": "", "MaxKernel": "", "MinKernel": ""}


def run(kexts, folders=("Lilu.kext", "AMFIPass.kext")):
    s = SETUP.read_text()
    assert s.count(START) == 1 and s.count(END) == 1, "anchors moved in nullmoth-setup.sh"
    t = tempfile.mkdtemp()
    oc = os.path.join(t, "EFI/OC")
    for f in folders:
        os.makedirs(os.path.join(oc, "Kexts", f))
    cfg = os.path.join(oc, "config.plist")
    with open(cfg, "wb") as fh:
        plistlib.dump({"Kernel": {"Add": kexts}}, fh)
    script = (f'C="{cfg}"; EDITS=(); has() {{ plutil -extract "$1" raw -o - "$C" >/dev/null 2>&1; }}\n'
              'get() { plutil -extract "$1" raw -o - "$C" 2>/dev/null; }\n'
              'stop() { echo "STOP $*"; exit 3; }\n'
              + s[s.index(START):s.index(END)]
              + '\necho "EDITS ${EDITS[*]+${EDITS[*]}}"; echo "NEEDAMFIPASS $NEEDAMFIPASS"\n')
    r = subprocess.run(["/bin/bash", "-c", script], capture_output=True, text=True, timeout=60)
    return r.returncode, r.stdout


class AmfiPass(unittest.TestCase):
    def test_amfipass_present_but_off_is_turned_on(self):
        rc, out = run([kext("Lilu.kext"), kext("AMFIPass.kext", False)])
        self.assertEqual(rc, 0, out)
        self.assertIn("CHANGE Kernel -> Add: turn AMFIPass.kext on", out)
        self.assertIn("EDITS Kernel.Add.1.Enabled|-bool|true", out)

    def test_negative_control_both_on_changes_nothing(self):
        rc, out = run([kext("Lilu.kext"), kext("AMFIPass.kext")])
        self.assertEqual(rc, 0, out)
        self.assertNotIn("CHANGE", out)
        self.assertIn("NEEDAMFIPASS 0", out)

    def test_missing_entry_with_the_kext_on_disk_is_added(self):
        rc, out = run([kext("Lilu.kext")])
        self.assertEqual(rc, 0, out)
        self.assertIn("NEEDAMFIPASS 1", out)

    def test_no_amfipass_anywhere_stops_with_what_to_do(self):
        rc, out = run([kext("Lilu.kext")], folders=("Lilu.kext",))
        self.assertEqual(rc, 3, out)
        self.assertIn("STOP AMFIPass.kext is not in this OpenCore EFI", out)

    def test_amfipass_before_lilu_stops(self):
        rc, out = run([kext("AMFIPass.kext"), kext("Lilu.kext")])
        self.assertEqual(rc, 3, out)
        self.assertIn("loads before Lilu.kext", out)


if __name__ == "__main__":
    unittest.main()
