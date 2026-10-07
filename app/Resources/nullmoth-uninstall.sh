#!/bin/bash
set -u
KEXTS="NVRM NVAccel NVRMFB NVRMAGDC"; EXT=/Library/Extensions; GB=/Library/GPUBundles
KC=/Library/KernelCollections/AuxiliaryKernelExtensions.kc
K=/System/Library/Kernels/kernel
KARG=(--allow-missing-kdk); [ -f "$K" ] && KARG+=(--kernel "$K")   # without a matching Kernel Debug Kit, kmutil needs --allow-missing-kdk
KB=/System/Library/KernelCollections/BootKernelExtensions.kc
KS=/System/Library/KernelCollections/SystemKernelExtensions.kc
die() { echo "STOP: $*" >&2; exit 1; }
[ "$(id -u)" -eq 0 ] || die "run with sudo"
BK=${1:-}
for k in $KEXTS; do rm -rf "$EXT/$k.kext"; done
for b in NVMTLDriver.bundle NVIDIAShared.bundle nvmtl nvmtl-allow.txt; do rm -rf "$GB/$b"; done
rm -rf /Library/NullMoth/kexts /Library/NullMoth/update-pending /Library/LaunchDaemons/com.nullmoth.osupdate.plist   # per-macOS NVAccel builds, update switcher
# 10-07 ("driver remove still isn't working for most people"; NM-VQB0RMVM kept an Oct 6 plugin after removing 1.0.5):
# the backup is taken right before an install, so on a Mac that already had an OLDER NullMoth driver it holds THAT
# driver - kexts, GPU bundles, and an auxiliary collection with com.nullmoth inside. Restoring it put the old driver
# back and exited before any check. Every kext and bundle named above is ours, so none is restored; the old collection
# is reused only when it holds none of our kexts, otherwise it is rebuilt below. The final check runs on every path.
restored=0
if [ -n "$BK" ]; then
  [ -d "$BK" ] || die "no backup at $BK"
  if [ -f "$BK/AuxiliaryKernelExtensions.kc" ]; then
    if kmutil inspect -a x86_64 -A "$BK/AuxiliaryKernelExtensions.kc" 2>/dev/null | grep -q com.nullmoth; then
      echo "The backup's kernel collection still holds an older NullMoth driver; rebuilding instead of restoring it."
    else cp -p "$BK/AuxiliaryKernelExtensions.kc" "$KC" && restored=1 && echo "Kernel collection restored from $BK."
    fi
  fi
fi
# 10-07 ("Remove NVIDIA driver didn't work"): with no third-party kext left, kmutil cannot build an auxiliary
# collection at all ("Cannot build collection without binaries", rc 31), so the uninstaller stopped and the OLD collection
# - with the NVIDIA driver inside - kept loading at every start. No kexts left = no auxiliary collection: remove it;
# macOS starts without one.
if [ "$restored" = 1 ]; then :
else
left=$(find "$EXT" -maxdepth 1 -name '*.kext' 2>/dev/null | while read -r x; do [ -d "$x/Contents/MacOS" ] && echo "$x"; done | wc -l | tr -d ' ')
if [ "$left" = 0 ]; then
  rm -f "$KC" "$KC".* && echo "No other kexts are installed; the auxiliary kernel collection was removed."
else
  kmutil create -n aux --volume-root / ${KARG[@]+"${KARG[@]}"} -B $KB -S $KS --repository "$EXT" -A "$KC" -z >/dev/null 2>&1 || die "kmutil could not rebuild the collection"
fi
fi
kmutil inspect -a x86_64 -A "$KC" 2>/dev/null | grep -q com.nullmoth && die "the kernel collection still lists the NVIDIA driver"
echo "Driver removed. Reboot now: sudo shutdown -r now"
