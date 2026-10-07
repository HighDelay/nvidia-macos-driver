#!/bin/bash
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
KEXTS="NVRM NVAccel NVRMFB NVRMAGDC"
EXT=/Library/Extensions; GB=/Library/GPUBundles; FW=/Users/Shared/nvfw
KC=/Library/KernelCollections/AuxiliaryKernelExtensions.kc
K=/System/Library/Kernels/kernel
# macOS 13+ kmutil refuses to build any kernel collection without a Kernel Debug Kit matching the build unless it is told
# --allow-missing-kdk. The auxiliary collection only links against the boot and system collections already on disk.
KARG=(--allow-missing-kdk); [ -f "$K" ] && KARG+=(--kernel "$K")
KB=/System/Library/KernelCollections/BootKernelExtensions.kc
KS=/System/Library/KernelCollections/SystemKernelExtensions.kc
BK=/Library/NullMoth/backup-$(date +%Y%m%d-%H%M%S)
step() { echo; echo "== $*"; }; ok() { echo "   ok  $*"; }; die() { echo "   STOP: $*" >&2; exit 1; }

[ "$(id -u)" -eq 0 ] || die "run with sudo"
step "1. this Mac"
[ "$(uname -m)" = x86_64 ] || die "Intel/x86_64 only"
v=$(sw_vers -productVersion); [ "${v%%.*}" = 15 ] || die "macOS 15 required (this is $v)"
ioreg -r -c IOPCIDevice -d 1 | grep -q '"vendor-id" = <de100000>' || die "no NVIDIA GPU found on PCI"
ok "macOS $v, x86_64, NVIDIA GPU present"

step "2. package integrity"
(cd "$HERE" && shasum -a 256 -c SHA256SUMS --quiet) || die "SHA256SUMS mismatch: re-download the package"
ok "every file matches SHA256SUMS"

step "3. test kernel collection"
T=$(mktemp -d /var/tmp/nullmoth.XXXX); mkdir -p "$T/repo"
for x in "$EXT"/*.kext; do n=$(basename "$x" .kext); case " $KEXTS " in *" $n "*) ;; *) cp -R "$x" "$T/repo/";; esac; done
for k in $KEXTS; do cp -R "$HERE/Library/Extensions/$k.kext" "$T/repo/" || die "copy $k"; done
kmutil create -n aux --volume-root / ${KARG[@]+"${KARG[@]}"} -B $KB -S $KS --repository "$T/repo" -A "$T/aux.kc" -z >"$T/kmutil.log" 2>&1
INS=$(kmutil inspect -a x86_64 -A "$T/aux.kc" 2>/dev/null)
for k in $KEXTS; do echo "$INS" | grep -q "com.nullmoth.$k" || { tail -20 "$T/kmutil.log"; die "kmutil refused com.nullmoth.$k (log above)"; }; done
ok "test collection holds all four kexts"
[ "${CHECK:-0}" = 1 ] && { rm -rf "$T"; echo; echo "CHECK PASS (nothing changed)"; exit 0; }

step "4. back up what is there now -> $BK"
mkdir -p "$BK"
for k in $KEXTS; do [ -e "$EXT/$k.kext" ] && cp -Rp "$EXT/$k.kext" "$BK/"; done
for b in NVMTLDriver.bundle NVIDIAShared.bundle nvmtl nvmtl-allow.txt; do [ -e "$GB/$b" ] && cp -Rp "$GB/$b" "$BK/"; done
[ -f "$KC" ] && cp -p "$KC" "$BK/AuxiliaryKernelExtensions.kc"
ok "backup written"

step "5. install"
for k in $KEXTS; do rm -rf "$EXT/$k.kext"; ditto "$HERE/Library/Extensions/$k.kext" "$EXT/$k.kext" || die "copy $k (restore from $BK)"; done
mkdir -p "$GB" "$FW"
for b in NVMTLDriver.bundle NVIDIAShared.bundle nvmtl; do rm -rf "$GB/$b"; ditto "$HERE/Library/GPUBundles/$b" "$GB/$b" || die "copy $b (restore from $BK)"; done
cp "$HERE/Library/GPUBundles/nvmtl-allow.txt" "$GB/"
ditto "$HERE/Users/Shared/nvfw" "$FW"
chown -R root:wheel "$EXT"/NV*.kext "$GB/NVMTLDriver.bundle" "$GB/NVIDIAShared.bundle" "$GB/nvmtl" "$GB/nvmtl-allow.txt"
chmod -R 755 "$EXT"/NV*.kext; chmod -R a+rX "$FW"
kmutil create -n aux --volume-root / ${KARG[@]+"${KARG[@]}"} -B $KB -S $KS --repository "$EXT" -A "$KC" -z >"$T/kmutil2.log" 2>&1
INS=$(kmutil inspect -a x86_64 -A "$KC" 2>/dev/null)
for k in $KEXTS; do echo "$INS" | grep -q "com.nullmoth.$k" || die "com.nullmoth.$k missing from the live collection (restore from $BK)"; done
rm -rf "$T"
ok "installed"

step "6. boot-args"
ba=$(nvram boot-args 2>/dev/null | cut -f2-)
for a in nvfb=1 nvaccel=1; do case " $ba " in *" $a "*) ;; *) echo "   NOTE: boot-args lack '$a' — add it in your OpenCore config.plist (see README)";; esac; done

echo; echo "Done. Reboot now: sudo shutdown -r now"
echo "If macOS asks, allow the extensions in System Settings > Privacy & Security, then reboot once more."
echo "Undo: sudo ./uninstall.sh $BK"
