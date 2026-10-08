#!/usr/bin/env python3
"""Ensure Tahoe preparation cannot park an already upgraded installation."""
from pathlib import Path
import subprocess
import unittest

root = Path(__file__).resolve().parents[1]
source = (root / 'app/Resources/nullmoth-setup.sh').read_text()
start = source.index('if [ "$UPD" = prepare ]; then\n  PREPARE_MAJOR=')
end = source.index('\n# "--update prepare"', start)
guard = source[start:end]
wait_start = source.index('    if [ "$UPD" = finish ] && [ "${FROM:-}" = "$MAJ" ]')
wait_condition = source[wait_start:source.index('; then', wait_start)]

class UpgradeGuard(unittest.TestCase):
    def run_guard(self, os_version, operation):
        script = 'sw_vers() { echo "$TEST_VERSION"; }\nstop() { echo "$*"; exit 1; }\n' + guard + '\necho allowed\n'
        return subprocess.run(['/bin/bash', '-c', script], capture_output=True, text=True,
                              env={'TEST_VERSION': os_version, 'UPD': operation})

    def test_prepare_allows_only_mac_os_15(self):
        self.assertEqual(self.run_guard('15.8.1', 'prepare').returncode, 0)
        for version in ['26.0.1', '14.8', '27.0', '']:
            result = self.run_guard(version, 'prepare')
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn('allowed', result.stdout)

    def test_cancel_and_finish_remain_available(self):
        for operation in ['cancel', 'finish']:
            self.assertEqual(self.run_guard('26.0.1', operation).returncode, 0)

    def test_guard_precedes_installer_and_efi_discovery(self):
        self.assertLess(start, source.index('INSTALL_THEN_PREPARE=0'))
        self.assertLess(start, source.index('step "Finding the OpenCore partition"'))

    def test_existing_tahoe_parking_is_restored_instead_of_waiting(self):
        for major, previous, waiting in [('15', '15', True), ('26', '15', False), ('26', '26', False)]:
            script = wait_condition + '; then echo wait; else echo finish; fi'
            result = subprocess.run(['/bin/bash', '-c', script], capture_output=True, text=True,
                                    env={'UPD': 'finish', 'MAJ': major, 'FROM': previous})
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.strip(), 'wait' if waiting else 'finish')

if __name__ == '__main__':
    unittest.main()
