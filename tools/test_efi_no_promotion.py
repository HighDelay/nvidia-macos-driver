"""Production selected-EFI block must never migrate another partition's boot files."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SOURCE = (ROOT / 'app/Resources/nullmoth-setup.sh').read_text()
START = SOURCE.index('  mount_efi "$EFI" || stop', SOURCE.index('  if [ "$EFI" = auto ]; then'))
END = SOURCE.index('\nif [ -n "$VERB" ]; then', START)
BLOCK = SOURCE[START:END]
PREAMBLE = r'''
EFI=selected-source; DRY=${FIXTURE_DRY:-0}; VERB=${FIXTURE_VERBOSE:-}; UPD=${FIXTURE_UPDATE:-}
step() { :; }
ok() { echo "OK $*"; }
note() { echo "NOTE $*"; }
stop() { echo "STOP $*"; exit 19; }
forbidden() { echo "FORBIDDEN $1"; exit 77; }
boot_esp() { forbidden boot_esp; }
mount_efi() {
  [ "$1" = selected-source ] || forbidden target-mount
  MOUNT_POINT=$FIXTURE_SOURCE
  printf '%s\n' "$1" >> "$FIXTURE_CALLS"
  if [ "${FIXTURE_APPEAR:-0}" = 1 ]; then
    /bin/mkdir -p "$FIXTURE_TARGET/EFI/Microsoft/Boot"
    printf 'concurrent foreign boot bytes' > "$FIXTURE_TARGET/EFI/Microsoft/Boot/BCD"
  fi
}
ocrel_in() { [ "$1" = "$FIXTURE_SOURCE" ] || forbidden target-discovery; echo "${FIXTURE_OCREL:-EFI/OC}"; }
on_usb() { [ "$1" = selected-source ] || forbidden other-usb-check; [ "${FIXTURE_USB:-1}" = 1 ]; }
mv() { forbidden mv; }
cp() { forbidden cp; }
rm() { forbidden rm; }
rmdir() { forbidden rmdir; }
mkdir() { forbidden mkdir; }
mktemp() { forbidden mktemp; }
ditto() { forbidden ditto; }
find() { forbidden find; }
plutil() { [ "$1" = -lint ] && [ "$2" = "$FIXTURE_SOURCE/${FIXTURE_OCREL:-EFI/OC}/config.plist" ] || forbidden wrong-config-lint; }
if true; then
'''

class SelectedEfi(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.source = self.root / 'source'; self.target = self.root / 'internal'
        (self.source / 'EFI/OC').mkdir(parents=True); self.target.mkdir()
        (self.source / 'EFI/OC/config.plist').write_bytes(b'selected source config')
        for rel in ('EFI/Microsoft/Boot/BCD', 'EFI/OtherVendor/loader.efi', 'EFI/BOOT/BOOTx64.efi'):
            p=self.source / rel; p.parent.mkdir(parents=True,exist_ok=True); p.write_bytes(rel.encode())
        self.calls=self.root/'calls'; self.script=self.root/'block.sh'
        self.script.write_text(PREAMBLE + BLOCK + '\necho "CONTINUE $EFI $C"\n')
        self.env=dict(os.environ,FIXTURE_SOURCE=str(self.source),FIXTURE_TARGET=str(self.target),FIXTURE_CALLS=str(self.calls))
    def snapshot(self):
        return {str(p.relative_to(self.root)):p.read_bytes() for tree in (self.source,self.target) for p in tree.rglob('*') if p.is_file() and not p.is_symlink()}
    def execute(self, **options):
        return subprocess.run(['/bin/bash',str(self.script)],env=dict(self.env,**options),capture_output=True,text=True,timeout=5)
    def check(self, options=None, guidance=True):
        before=self.snapshot(); result=self.execute(**(options or {}))
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertEqual(self.snapshot(),before)
        self.assertEqual(self.calls.read_text(),'selected-source\n')
        self.assertNotIn('FORBIDDEN',result.stdout)
        self.assertIn('CONTINUE selected-source '+str(self.source / 'EFI/OC/config.plist'),result.stdout)
        self.assertEqual('Keep it attached for every restart' in result.stdout,guidance)
    def test_missing_internal_efi_is_not_created(self):
        self.check(); self.assertFalse((self.target/'EFI').exists())
    def test_empty_internal_efi_is_not_removed(self):
        (self.target/'EFI').mkdir(); identity=(self.target/'EFI').stat().st_ino
        self.check(); self.assertEqual((self.target/'EFI').stat().st_ino,identity)
    def test_nonempty_internal_microsoft_vendor_and_fallback_bytes_remain(self):
        for rel in ('EFI/Microsoft/Boot/BCD','EFI/OtherVendor/loader.efi','EFI/BOOT/BOOTx64.efi','EFI/.foreign'):
            p=self.target/rel;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(b'unowned bytes')
        self.check()
    def test_existing_internal_oc_is_not_discovered_or_selected(self):
        p=self.target/'EFI/OC/config.plist';p.parent.mkdir(parents=True);p.write_bytes(b'other config')
        self.check()
    def test_internal_efi_symlink_is_not_followed(self):
        outside=self.root/'outside';outside.mkdir();(outside/'BCD').write_bytes(b'outside boot bytes')
        (self.target/'EFI').symlink_to(outside,target_is_directory=True)
        self.check();self.assertTrue((self.target/'EFI').is_symlink());self.assertEqual((outside/'BCD').read_bytes(),b'outside boot bytes')
    def test_dry_verbose_and_finish_keep_source_and_skip_install_guidance(self):
        for opts in ({'FIXTURE_DRY':'1'},{'FIXTURE_VERBOSE':'on'},{'FIXTURE_UPDATE':'finish'},{'FIXTURE_USB':'0'}):
            self.calls.unlink(missing_ok=True)
            with self.subTest(opts=opts):self.check(opts,guidance=False)
    def test_concurrent_foreign_content_is_never_read_moved_or_deleted(self):
        result=self.execute(FIXTURE_APPEAR='1')
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertEqual((self.target/'EFI/Microsoft/Boot/BCD').read_bytes(),b'concurrent foreign boot bytes')
        self.assertEqual((self.source/'EFI/OC/config.plist').read_bytes(),b'selected source config')
        self.assertFalse((self.target/'EFI/OC').exists())
        self.assertNotIn('FORBIDDEN',result.stdout)
    def test_explicit_source_fallback_config_continues_unchanged(self):
        p=self.source/'EFI/BOOT/config.plist';p.write_bytes(b'explicit fallback config')
        before=self.snapshot();result=self.execute(FIXTURE_OCREL='EFI/BOOT')
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertEqual(self.snapshot(),before)
        self.assertIn('CONTINUE selected-source '+str(p),result.stdout)

if __name__ == '__main__':
    unittest.main()
