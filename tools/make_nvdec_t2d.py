#!/usr/bin/env python3
"""Make libnvdec_h264_t2d.dylib, the H.264 decoder for GPUs before Blackwell, from libnvdec_h264.dylib.

The two differ in one byte: the untiling mode _emit_picture passes to nvd264_detile (movl $0x1,%edx at file offset
0x163d; 1 = Blackwell layout, 0 = the classic layout earlier NVDEC engines write). Issue #65 measured mode 0 on an
RTX 3070 Laptop (GA104): 0 of 393472 luma bytes differ from Apple's software decoder, 51 dB through VideoToolbox.
Refuses any other input library, so a rebuilt decoder cannot be patched blindly. Sign the output ad hoc after.

usage: make_nvdec_t2d.py <libnvdec_h264.dylib> <out libnvdec_h264_t2d.dylib>
"""
import hashlib
import sys

KNOWN = "ebf78288"          # SHA-256 prefix of the decoder this offset was read from (1.0.6 - 1.10 packages)
OFFSET = 0x163D             # movl $imm32, %edx before callq _nvd264_detile


def main(src, dst):
    data = bytearray(open(src, "rb").read())
    if not hashlib.sha256(data).hexdigest().startswith(KNOWN):
        raise SystemExit(f"{src} is not the known decoder ({KNOWN}...): read the mode offset again before patching")
    if data[OFFSET:OFFSET + 5] != bytes([0xBA, 1, 0, 0, 0]):
        raise SystemExit(f"unexpected bytes at {OFFSET:#x}: {data[OFFSET:OFFSET + 5].hex()}")
    data[OFFSET + 1] = 0
    with open(dst, "wb") as f:
        f.write(data)
    print(f"wrote {dst}: untiling mode 1 -> 0")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    main(sys.argv[1], sys.argv[2])
