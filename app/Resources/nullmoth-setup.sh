#!/bin/bash
PATH=/usr/bin:/bin:/usr/sbin:/sbin
B=7C436110-AB2A-4BBB-A880-FE41995C9F82
WANT_ARGS="nvfb=1 nvaccel=1 nvfbheads=4 -nvkmsnosmooth amfi_get_out_of_my_way=0x1 amfi=0x80"
DROP_ARGS="nv_disable=1 -wegnoegpu"          # both hide the NVIDIA card from macOS
SIP_BITS=$((0x0A43))
TOOL_NAME="1401: Remove NVIDIA driver"; TOOL_FILE=NullMothSafe.efi
ST=${NULLMOTH_STATE_DIR:-/Library/NullMoth}; STATE=$ST/state
AGENT=/Library/LaunchAgents/com.nullmoth.crashcheck.plist
RECOVER=/Library/LaunchDaemons/com.nullmoth.recover.plist
VERB=""; PKG=""; SHA=""; CFG=""; EFI=auto; DRY=0; REMOVE=0; USBMAP=""; TOOL=""; APPBIN=""; MOUNTED=""; T=""
while [ $# -gt 0 ]; do case "$1" in
  --pkg) PKG=$2; shift;; --sha) SHA=$2; shift;; --config) CFG=$2; shift;; --efi) EFI=$2; shift;;
  --tool) TOOL=$2; shift;; --usbmap) USBMAP=$2; shift;; --app) APPBIN=$2; shift;; --dry) DRY=1;; --remove) REMOVE=1;; --verbose) VERB=$2; shift;;
  *) echo "STOP unknown option $1"; echo "RESULT stop"; exit 2;; esac; shift; done
step() { echo "STEP $*"; }; ok() { echo "OK $*"; }; note() { echo "NOTE $*"; }
cleanup() { for d in $MOUNTED; do diskutil unmount "$d" >/dev/null 2>&1; done; [ -n "$T" ] && rm -rf "$T"; }
stop() { echo "STOP $*"; cleanup; echo "RESULT stop"; exit 1; }
[ "$(id -u)" = 0 ] || { echo "STOP needs administrator rights"; echo "RESULT stop"; exit 1; }
has() { plutil -extract "$1" raw -o - "$C" >/dev/null 2>&1; }
get() { plutil -extract "$1" raw -o - "$C" 2>/dev/null; }
mnt() { diskutil info "$1" 2>/dev/null | awk -F': *' '/Mount Point/{print $2}'; }
mount_efi() {   # $1 = device or partition UUID; prints the mount point
  local mp; mp=$(mnt "$1")
  if [ -z "$mp" ]; then diskutil mount "$1" >/dev/null 2>&1 || return 1; MOUNTED="$MOUNTED $1"; mp=$(mnt "$1"); fi
  echo "$mp"; }
ocrel_in() {    # $1 = mount point; prints where OpenCore lives on it (EFI/OC, or EFI/BOOT when OpenCore.efi is BOOTx64.efi)
  # A config made for another Mac model (a rescue stick, another machine's EFI) is never this Mac's: OpenCore sets the
  # model macOS reports from PlatformInfo, so the config that started this Mac names hw.model. (Measured 10-07: the boot-path
  # variable was absent and a Mac Pro rescue config on a second stick was edited instead of the 1401 stick's.)
  local d m; m=$(sysctl -n hw.model 2>/dev/null)
  for d in EFI/OC EFI/BOOT; do
    [ -f "$1/$d/config.plist" ] || continue
    [ "$d" = EFI/BOOT ] && [ ! -d "$1/$d/Kexts" ] && continue
    local c="$1/$d/config.plist" cm="" k
    # the model this config gives the Mac: SMBIOS spoof (Generic / SMBIOS / DataHub), else OCLP's own record of a real Mac
    for k in PlatformInfo.Generic.SystemProductName PlatformInfo.SMBIOS.SystemProductName PlatformInfo.DataHub.SystemProductName \
             NVRAM.Add.4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102.OCLP-Model; do
      cm=$(plutil -extract "$k" raw -o - "$c" 2>/dev/null); [ -n "$cm" ] && break; done
    [ -n "$m" ] && [ -n "$cm" ] && [ "$cm" != "$m" ] && continue
    echo $d; return; done; }
booted_part() { # the partition OpenCore started this Mac from, read from OpenCore's boot-path variable
  local bp u d
  bp=$(nvram 4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:boot-path 2>/dev/null | cut -f2-)
  u=$(printf '%s' "$bp" | grep -oiE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
  [ -n "$u" ] || return 1
  for d in $(diskutil list | grep -oE 'disk[0-9]+s[0-9]+$' | sort -u); do
    diskutil info "$d" 2>/dev/null | grep -qi "Partition UUID: *$u" && { echo "$d"; return 0; }; done
  return 1; }
tool_index() {  # index of our entry in Misc.Tools, or nothing
  local i=0 p
  while p=$(plutil -extract Misc.Tools.$i.Path raw -o - "$C" 2>/dev/null); do
    [ "$p" = "$TOOL_FILE" ] && { echo $i; return; }; i=$((i+1)); done; }
csr_of() { local h; h=$(get NVRAM.Add.$B.csr-active-config | base64 -D 2>/dev/null | xxd -p)
  [ ${#h} = 8 ] && echo $(( 0x${h:6:2}${h:4:2}${h:2:2}${h:0:2} )) || echo 0; }
csr_data() { printf '%08x' "$1" | sed -E 's/(..)(..)(..)(..)/\4\3\2\1/' | xxd -r -p | base64; }
bidx() { local i=0 p; while p=$(plutil -extract Kernel.Block.$i.Identifier raw -o - "$C" 2>/dev/null); do [ "$p" = com.apple.iokit.IONDRVSupport ] && { echo $i; return; }; i=$((i+1)); done; }

if [ $REMOVE = 1 ]; then
  step "Removing the NullMoth driver"
  [ -f "$STATE" ] || stop "no install record in $STATE - was the driver installed by this app?"
  . "$STATE"
  if [ "${NULLMOTH_CONFIG_ONLY:-0}" != 1 ]; then
    [ -x $ST/uninstall.sh ] || stop "no uninstaller in $ST"
    $ST/uninstall.sh "${DRIVER_BACKUP:-}" 2>&1 | sed 's/^/NOTE /'
    rc=${PIPESTATUS[0]}; [ "$rc" = 0 ] || stop "the driver uninstaller failed (exit $rc)"
    ok "driver files removed and the kernel collection rebuilt"
    rm -f "$AGENT"
  rm -f "$RECOVER" "$ST/nullmoth-recover.sh"
  fi
  if [ -n "${EFI_UUID:-}" ] || [ -n "$CFG" ]; then
    step "Undoing the OpenCore changes"
    if [ -n "$CFG" ]; then C=$CFG; MP=$(cd "$(dirname "$C")/../.." && pwd)
    else MP=$(mount_efi "$EFI_UUID") || stop "could not mount the OpenCore partition $EFI_UUID - restore $CONFIG_BACKUP by hand"
      C="$MP/${OCREL:-EFI/OC}/config.plist"; fi
    if [ "$(shasum -a 256 "$C" | awk '{print $1}')" = "$CONFIG_SHA_AFTER" ] && [ -f "$MP/$CONFIG_BACKUP_REL" ]; then
      cp -p "$MP/$CONFIG_BACKUP_REL" "$C" && ok "config restored from the backup taken before the install"
    else
      note "the config changed after the install, so only NullMoth's own edits are undone"
      args=$(get NVRAM.Add.$B.boot-args); new=""
      for a in $args; do case " $ADDED_ARGS " in *" $a "*) ;; *) new="$new $a";; esac; done
      for a in $REMOVED_ARGS; do new="$new $a"; done
      plutil -replace NVRAM.Add.$B.boot-args -string "${new# }" "$C"
      [ "$(csr_of)" = "${NEW_CSR:-x}" ] && plutil -replace NVRAM.Add.$B.csr-active-config -data "$(csr_data "$OLD_CSR")" "$C"
      [ -n "${OLD_SBM:-}" ] && [ "$(get Misc.Security.SecureBootModel)" = Disabled ] && plutil -replace Misc.Security.SecureBootModel -string "$OLD_SBM" "$C"
      i=$(tool_index); [ -n "$i" ] && plutil -remove Misc.Tools.$i "$C"
      # back to the installer-safe settings 1401 wrote: small BAR for macOS, firmware framebuffer allowed
      plutil -replace UEFI.Quirks.ResizeGpuBars -integer -1 "$C"; plutil -replace Booter.Quirks.ResizeAppleGpuBars -integer 0 "$C"
      bi=$(bidx 2>/dev/null); [ -n "$bi" ] && plutil -replace Kernel.Block.$bi.Enabled -bool false "$C"
      plutil -lint "$C" >/dev/null || { cp -p "$MP/$CONFIG_BACKUP_REL" "$C"; note "the edited config failed to lint; restored the pre-install backup instead"; }
      ok "NullMoth's OpenCore edits undone"
    fi
    rm -f "$MP/${OCREL:-EFI/OC}/Tools/$TOOL_FILE"
  fi
  mv "$STATE" "$STATE.removed-$(date +%Y%m%d-%H%M%S)"
  ok "done - restart to finish"; cleanup; echo "RESULT ok"; exit 0
fi

if [ -z "$USBMAP" ] && [ -z "$VERB" ]; then
step "Checking the driver package"
[ -f "$PKG" ] || stop "package not found: $PKG"
got=$(shasum -a 256 "$PKG" | awk '{print $1}')
[ "$got" = "$SHA" ] || stop "package checksum $got does not match $SHA - download it again"
ok "package matches its SHA-256"
[ -n "$TOOL" ] && [ -f "$TOOL" ] || stop "the boot-picker tool is missing from the app"
fi

if [ -n "$CFG" ]; then C=$CFG; [ -f "$C" ] || stop "no config at $C"; MP=$(cd "$(dirname "$C")/../.." && pwd); OCREL="EFI/$(basename "$(dirname "$C")")"; ok "OpenCore config: $C (given)"
else
  step "Finding the OpenCore partition"
  if [ "$EFI" = auto ]; then
    found=""
    if d=$(booted_part); then mp=$(mount_efi "$d") && [ -n "$(ocrel_in "$mp")" ] && { found=$d; ok "OpenCore started this Mac from $d"; }; fi
    if [ -z "$found" ]; then
      # OpenCore can live on an EFI partition or on any FAT32 partition (a 1401 stick is a FAT32 data partition)
      for d in $(diskutil list | awk '/ EFI | DOS_FAT_32 | Windows_FAT_32 | Microsoft Basic Data /{print $NF}' | grep -E '^disk[0-9]+s[0-9]+$'); do
        mp=$(mount_efi "$d") || continue; [ -n "$(ocrel_in "$mp")" ] && found="$found $d"; done
    fi
    n=$(echo $found | wc -w | tr -d ' ')
    [ "$n" = 0 ] && stop "no OpenCore config for this Mac ($(sysctl -n hw.model)) on any connected disk - plug in the disk or USB stick OpenCore started from, then try again"
    [ "$n" -gt 1 ] && { for d in $found; do echo "NOTE candidate $d"; done; stop "several OpenCore partitions found - pick one"; }
    EFI=${found# }
  fi
  MP=$(mount_efi "$EFI") || stop "could not mount $EFI"
  OCREL=$(ocrel_in "$MP"); [ -n "$OCREL" ] || stop "$EFI has no OpenCore config (EFI/OC/config.plist or EFI/BOOT/config.plist)"
  C="$MP/$OCREL/config.plist"
  ok "OpenCore config: $EFI ($C)"
fi
plutil -lint "$C" >/dev/null || stop "$C is not a valid plist - fix it before installing"

if [ -n "$VERB" ]; then
  # Verbose startup = boot argument -v. It goes in the config (OpenCore rewrites boot-args every boot when NVRAM Delete
  # lists it) AND in NVRAM now (a config without that Delete entry keeps whatever NVRAM already holds).
  case "$VERB" in on|off) ;; *) stop "--verbose takes on or off";; esac
  step "Turning verbose startup $VERB"
  BKC="$C.nullmoth-verbose-$(date +%Y%m%d-%H%M%S)"; cp -p "$C" "$BKC" || stop "could not back up $C"
  args=$(get NVRAM.Add.$B.boot-args); new=""
  for a in $args; do [ "$a" = -v ] || new="$new $a"; done
  [ "$VERB" = on ] && new="$new -v"; new=${new# }
  if has NVRAM.Add.$B.boot-args; then plutil -replace NVRAM.Add.$B.boot-args -string "$new" "$C"
  else plutil -insert NVRAM.Add.$B.boot-args -string "$new" "$C"; fi
  plutil -lint "$C" >/dev/null || { cp -p "$BKC" "$C"; stop "editing the config failed - the original is restored"; }
  ok "OpenCore boot-args: $new"
  cur=$(nvram boot-args 2>/dev/null | cut -f2-); nv=""
  for a in $cur; do [ "$a" = -v ] || nv="$nv $a"; done
  [ "$VERB" = on ] && nv="$nv -v"; nv=${nv# }
  nvram boot-args="$nv" && ok "NVRAM boot-args: $nv" || note "could not write NVRAM boot-args; the config change still applies"
  ok "verbose startup $VERB - takes effect at the next restart"; cleanup; echo "RESULT ok"; exit 0
fi

if [ -n "$USBMAP" ]; then
  step "Installing the USB map"
  K="$USBMAP/UTBMap.kext"; plutil -lint "$K/Contents/Info.plist" >/dev/null || stop "the USB map the app wrote is not valid"
  KD="$(dirname "$C")/Kexts"; [ -d "$KD/USBToolBox.kext" ] || stop "USBToolBox.kext is not in $KD - the map needs it (1401 builds include it)"
  kidx() { local i=0 p; while p=$(plutil -extract Kernel.Add.$i.BundlePath raw -o - "$C" 2>/dev/null); do [ "$p" = "$1" ] && { echo $i; return; }; i=$((i+1)); done; }
  [ -n "$(kidx USBToolBox.kext)" ] || stop "USBToolBox.kext is not in the config's Kernel -> Add"
  BKC="$C.nullmoth-usb-$(date +%Y%m%d-%H%M%S)"; cp -p "$C" "$BKC" || stop "could not back up $C"; ok "backed up the config to $BKC"
  [ -d "$KD/UTBMap.kext" ] && { mv "$KD/UTBMap.kext" "$KD/UTBMap.kext.nullmoth-$(date +%Y%m%d-%H%M%S)" || stop "could not move the old map aside"; note "the old UTBMap.kext was kept beside it"; }
  cp -R "$K" "$KD/UTBMap.kext" || { cp -p "$BKC" "$C"; stop "could not copy the map"; }
  fail=0
  i=$(kidx USBToolBox.kext); plutil -replace Kernel.Add.$i.Enabled -bool true "$C" || fail=1
  i=$(kidx UTBDefault.kext); [ -n "$i" ] && { plutil -replace Kernel.Add.$i.Enabled -bool false "$C" || fail=1; echo "CHANGE UTBDefault.kext: off (the all-ports map)"; }
  i=$(kidx UTBMap.kext)
  if [ -n "$i" ]; then plutil -replace Kernel.Add.$i.Enabled -bool true "$C" || fail=1
  else plutil -insert Kernel.Add -json '{"Arch":"Any","BundlePath":"UTBMap.kext","Comment":"NullMoth USB map","Enabled":true,"ExecutablePath":"","MaxKernel":"","MinKernel":"","PlistPath":"Contents/Info.plist"}' -append "$C" || fail=1
    echo "CHANGE Kernel -> Add: UTBMap.kext (after USBToolBox.kext)"; fi
  if [ $fail = 1 ] || ! plutil -lint "$C" >/dev/null; then cp -p "$BKC" "$C"; rm -rf "$KD/UTBMap.kext"; stop "editing the config failed - the original is restored"; fi
  ok "USB map installed - restart to use it"; cleanup; echo "RESULT ok"; exit 0
fi

if [ $DRY = 0 ]; then
  step "Test-building the driver (nothing is changed yet)"
  T=$(mktemp -d /var/tmp/nullmoth.XXXX)
  tar -xzf "$PKG" -C "$T" || stop "could not unpack the package"
  [ -x "$T/pkgroot/install.sh" ] || stop "the package has no install.sh"
  out=$(cd "$T/pkgroot" && CHECK=1 ./install.sh 2>&1); rc=$?
  echo "$out" | sed -E '/^$/d;s/^== /NOTE /;s/^   ok  /NOTE /;s/^   STOP: /STOP /;s/^   NOTE: /NOTE /'
  [ "$rc" = 0 ] || stop "the driver test build failed (exit $rc) - OpenCore and macOS were not changed"
  rm -rf "$T"; T=""
fi
# SIP is read from the running system: a config value only counts once OpenCore has applied it at a boot.
SIPON=0; csrutil status 2>/dev/null | grep -q 'status: enabled\.' && SIPON=1

step "Checking the OpenCore settings"
EDITS=(); ADDED=""; REMOVED=""
args=$(get NVRAM.Add.$B.boot-args); new=""
for a in $args; do case " $DROP_ARGS " in *" $a "*) echo "CHANGE boot-args: remove $a (it hides the NVIDIA card)"; REMOVED="$REMOVED $a";; *) new="$new $a";; esac; done
for a in $WANT_ARGS; do case " $new " in *" $a "*) ;; *) new="$new $a"; ADDED="$ADDED $a"; echo "CHANGE boot-args: add $a";; esac; done
new=${new# }
[ "$new" != "$args" ] && EDITS+=("NVRAM.Add.$B.boot-args|-string|$new")
cur=$(csr_of); want=$(( cur | SIP_BITS ))
if [ $want != $cur ]; then
  echo "CHANGE csr-active-config: $(printf '0x%04X' $cur) -> $(printf '0x%04X' $want) (the SIP bits the driver needs)"
  EDITS+=("NVRAM.Add.$B.csr-active-config|-data|$(csr_data $want)")
fi
sbm=$(get Misc.Security.SecureBootModel); OLDSBM=""
if [ "$sbm" != Disabled ]; then
  echo "CHANGE SecureBootModel: ${sbm:-unset} -> Disabled (Apple Secure Boot refuses kexts Apple did not sign)"
  EDITS+=("Misc.Security.SecureBootModel|-string|Disabled"); OLDSBM=${sbm:-Default}
fi
# The macOS installer boots with a small GPU BAR (ResizeAppleGpuBars 0, so its fallback screen survives PCI setup); the
# driver was tested with the card's full 8 GB BAR, so the installed system gets that back.
bar=$(get UEFI.Quirks.ResizeGpuBars); abar=$(get Booter.Quirks.ResizeAppleGpuBars)
# 13 = 8 GB: the full BAR of an 8 GB card, measured 10-07 on the RTX 5060 (NVRM moves BAR1 out of the console, display armed).
[ "$bar" != 13 ] && { echo "CHANGE ResizeGpuBars: ${bar:-unset} -> 13 (8 GB BAR, full memory bandwidth)"; EDITS+=("UEFI.Quirks.ResizeGpuBars|-integer|13"); }
[ "$abar" != -1 ] && { echo "CHANGE ResizeAppleGpuBars: ${abar:-unset} -> -1 (macOS sees the full BAR)"; EDITS+=("Booter.Quirks.ResizeAppleGpuBars|-integer|-1"); }
# The installer runs on the firmware framebuffer (IONDRVSupport); once the driver is in, that framebuffer would take
# display index 0 from NVRMFB, so the installed system excludes it (as on the tested RTX 5060 setup).
NEEDBLOCK=0; bi=$(bidx)
if [ -z "$bi" ]; then NEEDBLOCK=1; echo "CHANGE Kernel -> Block: exclude IONDRVSupport (the firmware framebuffer would take NVRMFB's display)"
elif [ "$(get Kernel.Block.$bi.Enabled)" != true ]; then EDITS+=("Kernel.Block.$bi.Enabled|-bool|true"); echo "CHANGE Kernel -> Block: turn the IONDRVSupport exclude on"; fi
del=$(plutil -extract NVRAM.Delete.$B xml1 -o - "$C" 2>/dev/null); DELADD=()
for k in boot-args csr-active-config; do echo "$del" | grep -q "<string>$k</string>" || { DELADD+=("$k"); echo "CHANGE NVRAM Delete: add $k (so OpenCore rewrites it every boot)"; }; done
NEEDTOOL=0; [ -z "$(tool_index)" ] && { NEEDTOOL=1; echo "CHANGE boot picker: add \"$TOOL_NAME\" (the way back if the driver ever stops macOS starting)"; }
[ ${#EDITS[@]} = 0 ] && [ ${#DELADD[@]} = 0 ] && [ $NEEDTOOL = 0 ] && [ $NEEDBLOCK = 0 ] && ok "OpenCore already has every setting the driver needs"

if [ $SIPON = 1 ]; then
  note "SIP is still fully on in this boot: only SIP, Secure Boot, boot arguments and the boot picker entry change now"
  KEEP=(); for e in "${EDITS[@]}"; do case "$e" in UEFI.Quirks.*|Booter.Quirks.*|Kernel.Block.*) ;; *) KEEP+=("$e");; esac; done
  EDITS=("${KEEP[@]+"${KEEP[@]}"}"); NEEDBLOCK=0
fi
if [ $DRY = 0 ]; then
  BKC="$C.nullmoth-$(date +%Y%m%d-%H%M%S)"
  cp -p "$C" "$BKC" || stop "could not back up $C"
  ok "backed up the config to $BKC"
  fail=0
  for e in "${EDITS[@]}"; do IFS='|' read -r k t v <<<"$e"
    if has "$k"; then plutil -replace "$k" $t "$v" "$C" || fail=1; else plutil -insert "$k" $t "$v" "$C" || fail=1; fi; done
  if [ $NEEDBLOCK = 1 ]; then
    has Kernel.Block || plutil -insert Kernel.Block -array "$C" || fail=1
    plutil -insert Kernel.Block -json '{"Arch":"Any","Comment":"boot framebuffer IONDRVFramebuffer steals index 0 from NVRMFB","Enabled":true,"Identifier":"com.apple.iokit.IONDRVSupport","MaxKernel":"","MinKernel":"","Strategy":"Exclude"}' -append "$C" || fail=1
  fi
  if [ ${#DELADD[@]} -gt 0 ]; then
    has NVRAM.Delete || plutil -insert NVRAM.Delete -dictionary "$C" || fail=1
    plutil -extract NVRAM.Delete.$B xml1 -o - "$C" >/dev/null 2>&1 || plutil -insert NVRAM.Delete.$B -array "$C" || fail=1
    for k in "${DELADD[@]}"; do plutil -insert NVRAM.Delete.$B -string "$k" -append "$C" || fail=1; done
  fi
  if [ $NEEDTOOL = 1 ]; then
    plutil -extract Misc.Tools xml1 -o - "$C" >/dev/null 2>&1 || plutil -insert Misc.Tools -array "$C" || fail=1
    plutil -insert Misc.Tools -json "{\"Arguments\":\"\",\"Auxiliary\":false,\"Comment\":\"NullMoth: remove the NVIDIA driver at the next start\",\"Enabled\":true,\"Flavour\":\"Auto\",\"FullNvramAccess\":true,\"Name\":\"$TOOL_NAME\",\"Path\":\"$TOOL_FILE\",\"RealPath\":false,\"TextMode\":false}" -append "$C" || fail=1
    mkdir -p "$MP/$OCREL/Tools" && cp "$TOOL" "$MP/$OCREL/Tools/$TOOL_FILE" || fail=1
  fi
  if [ $fail = 1 ] || ! plutil -lint "$C" >/dev/null; then cp -p "$BKC" "$C"; stop "editing the config failed - the original is restored"; fi
  ok "OpenCore config updated"
  UUID=""; [ -z "$CFG" ] && UUID=$(diskutil info "$EFI" | awk -F': *' '/Partition UUID/{print $2}')
  mkdir -p "$ST"
  { echo "# NullMoth install record $(date -u +%Y-%m-%dT%H:%M:%SZ) - read by nullmoth-setup.sh --remove"
    echo "EFI_UUID='$UUID'"; echo "OCREL='$OCREL'"; echo "CONFIG_BACKUP_REL='$OCREL/$(basename "$BKC")'"; echo "CONFIG_BACKUP='$BKC'"
    echo "CONFIG_SHA_AFTER='$(shasum -a 256 "$C" | awk '{print $1}')'"
    echo "ADDED_ARGS='${ADDED# }'"; echo "REMOVED_ARGS='${REMOVED# }'"
    echo "OLD_CSR='$cur'"; echo "NEW_CSR='$want'"; echo "OLD_SBM='$OLDSBM'"; } > "$STATE.tmp" && mv "$STATE.tmp" "$STATE"
fi
[ "${NULLMOTH_CONFIG_ONLY:-0}" = 1 ] && { cleanup; echo "RESULT ok"; exit 0; }
if [ $SIPON = 1 ] && [ $DRY = 0 ]; then
  rm -f "$STATE"
  ok "SIP setting written - restart, then open 1401 again to install the driver"; cleanup; echo "RESULT ok"; exit 0
fi

step "Installing the driver"
T=$(mktemp -d /var/tmp/nullmoth.XXXX)
tar -xzf "$PKG" -C "$T" || stop "could not unpack the package"
[ -x "$T/pkgroot/install.sh" ] || stop "the package has no install.sh"
if [ $DRY = 1 ]; then out=$(cd "$T/pkgroot" && CHECK=1 ./install.sh 2>&1); rc=$?
else out=$(cd "$T/pkgroot" && ./install.sh 2>&1); rc=$?; fi
echo "$out" | sed -E '/^$/d;s/^== /NOTE /;s/^   ok  /NOTE /;s/^   STOP: /STOP /;s/^   NOTE: /NOTE /'
if [ "$rc" != 0 ]; then
  [ $DRY = 0 ] && [ -n "${BKC:-}" ] && [ -f "$BKC" ] && cp -p "$BKC" "$C" && rm -f "$STATE" && note "OpenCore config put back as it was before"
  stop "the driver installer stopped (exit $rc)"
fi
if [ $DRY = 1 ]; then ok "dry run: the driver would install cleanly (nothing was changed)"; cleanup; echo "RESULT ok"; exit 0; fi

mkdir -p $ST && cp "$T/pkgroot/uninstall.sh" $ST/ && chmod 755 $ST/uninstall.sh
DBK=$(echo "$out" | sed -n 's/^Undo: sudo .\/uninstall.sh //p' | tail -1)
echo "DRIVER_BACKUP='$DBK'" >> "$STATE"
ok "install record written to $STATE"
if [ -n "$APPBIN" ] && [ -x "$APPBIN" ]; then
  cat > "$AGENT" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.nullmoth.crashcheck</string>
<key>ProgramArguments</key><array><string>$APPBIN</string><string>--crash-check</string></array>
<key>RunAtLoad</key><true/>
<key>ProcessType</key><string>Background</string>
</dict></plist>
PL
  chmod 644 "$AGENT"; ok "crash check added (asks before sending anything)"
fi
cp "$0" "$ST/nullmoth-setup.sh" && chmod 755 "$ST/nullmoth-setup.sh"
cat > "$ST/nullmoth-recover.sh" <<'RS'
#!/bin/bash
PATH=/usr/bin:/bin:/usr/sbin:/sbin
V=7C436110-AB2A-4BBB-A880-FE41995C9F82:nullmoth-remove
nvram "$V" >/dev/null 2>&1 || exit 0
nvram -d "$V"
{ date; /bin/bash /Library/NullMoth/nullmoth-setup.sh --remove; } >> /Library/NullMoth/recover.log 2>&1
sleep 2; /sbin/shutdown -r now
RS
chmod 755 "$ST/nullmoth-recover.sh"
cat > "$RECOVER" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>com.nullmoth.recover</string>
<key>ProgramArguments</key><array><string>/bin/bash</string><string>$ST/nullmoth-recover.sh</string></array>
<key>RunAtLoad</key><true/>
</dict></plist>
PL
chmod 644 "$RECOVER"; chown root:wheel "$RECOVER"
ok "boot picker way back armed (NullMoth: Remove driver)"
ok "driver installed - restart to load it"
cleanup; echo "RESULT ok"
