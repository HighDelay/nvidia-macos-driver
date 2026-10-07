#!/usr/bin/env python3
"""vtdump.py BINARY [CLASS...] — print each C++ vtable in an x86_64 kext as slot -> method name.
Reads classic Mach-O relocations: a slot with an external relocation names the imported symbol; any other slot holds the
target address, resolved through the symbol table. Kexts are linked at 0, so a relocation address is a vm address."""
import struct, sys, subprocess
def load(path):
    d = open(path, 'rb').read()
    if struct.unpack_from('<I', d, 0)[0] == 0xcafebabe:            # fat: take the x86_64 slice
        n = struct.unpack_from('>I', d, 4)[0]
        for i in range(n):
            ct, _, off, size, _ = struct.unpack_from('>IIIII', d, 8 + i * 20)
            if ct == 0x01000007: d = d[off:off + size]; break
    ncmds = struct.unpack_from('<I', d, 16)[0]; p = 32; segs = []; sym = dsym = None
    for _ in range(ncmds):
        cmd, sz = struct.unpack_from('<II', d, p)
        if cmd == 0x19:
            vm, vms, fo, fs = struct.unpack_from('<QQQQ', d, p + 24); segs.append((vm, vms, fo, fs))
        elif cmd == 0x2: sym = struct.unpack_from('<IIII', d, p + 8)
        elif cmd == 0xb: dsym = struct.unpack_from('<18I', d, p + 8)
        p += sz
    symoff, nsyms, stroff, _ = sym
    syms = []
    for i in range(nsyms):
        strx, typ, sect, desc, val = struct.unpack_from('<IBBHQ', d, symoff + i * 16)
        e = d.index(b'\0', stroff + strx); syms.append((d[stroff + strx:e].decode(errors='replace'), typ, sect, val))
    extrel = {}
    extreloff, nextrel = dsym[14], dsym[15]
    for i in range(nextrel):
        addr, info = struct.unpack_from('<iI', d, extreloff + i * 8)
        extrel[addr] = syms[info & 0xffffff][0]
    def fo(vm):
        for v, vs, f, fs in segs:
            if v <= vm < v + vs: return f + (vm - v) if vm - v < fs else None
    byaddr = {}
    for n, t, s, v in syms:
        if s and n and not (t & 0xe0): byaddr.setdefault(v, n)
    return d, syms, extrel, fo, byaddr
def vtables(path, want=None):
    d, syms, extrel, fo, byaddr = load(path)
    defined = sorted((v, n) for n, t, s, v in syms if s and n and not (t & 0xe0))
    out = {}
    for i, (v, n) in enumerate(defined):
        if not n.startswith('__ZTV'): continue
        cls = n[5:]
        if want and cls not in want: continue
        end = next((w for w, _ in defined[i + 1:] if w > v), v + 8 * 400)
        slots = []
        for a in range(v + 16, end, 8):                         # skip offset-to-top and typeinfo
            if a in extrel: slots.append(extrel[a]); continue
            off = fo(a); val = struct.unpack_from('<Q', d, off)[0] if off is not None else 0
            slots.append(byaddr.get(val, '0x%x' % val) if val else '0')
        while slots and slots[-1] == '0': slots.pop()
        out[cls] = slots
    return out
if __name__ == '__main__':
    vt = vtables(sys.argv[1], set(sys.argv[2:]) or None)
    names = sorted({s for v in vt.values() for s in v})
    dm = dict(zip(names, subprocess.run(['c++filt'], input='\n'.join(names), capture_output=True, text=True).stdout.split('\n')))
    for c, s in vt.items():
        print('## %s %d' % (c, len(s)))
        for i, x in enumerate(s): print('%3d %s' % (i, dm.get(x, x)))
