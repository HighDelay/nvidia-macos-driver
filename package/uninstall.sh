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
if [ -n "$BK" ]; then
  [ -d "$BK" ] || die "no backup at $BK"
  for k in $KEXTS; do [ -e "$BK/$k.kext" ] && ditto "$BK/$k.kext" "$EXT/$k.kext"; done
  for b in NVMTLDriver.bundle NVIDIAShared.bundle nvmtl nvmtl-allow.txt; do [ -e "$BK/$b" ] && ditto "$BK/$b" "$GB/$b"; done
  [ -f "$BK/AuxiliaryKernelExtensions.kc" ] && cp -p "$BK/AuxiliaryKernelExtensions.kc" "$KC" && { echo "Restored from $BK. Reboot now."; exit 0; }
fi
kmutil create -n aux --volume-root / ${KARG[@]+"${KARG[@]}"} -B $KB -S $KS --repository "$EXT" -A "$KC" -z >/dev/null 2>&1 || die "kmutil could not rebuild the collection"
echo "Driver removed. Reboot now: sudo shutdown -r now"
