/*
 * IOAccelerator — reconstructed from IOGraphicsFamily (macOS Sequoia, x86_64).
 *
 * The base of IOGraphicsAccelerator2, which is the class our driver must BE.
 * Apple ships no header.
 *
 * THIS CLASS DOES NOT LIVE IN IOAcceleratorFamily2. It is UNDEFINED there
 * (`nm` shows `U __ZN13IOAccelerator10gMetaClassE`). It lives in
 * IOGraphicsFamily, which Sequoia ships ONLY inside SystemKernelExtensions.kc —
 * there is no standalone binary on disk. Read it with:
 *   vtable.py <kc> --entry com.apple.iokit.IOGraphicsFamily --class IOAccelerator
 *
 * WHERE EACH FACT CAME FROM
 *   size        136 bytes — from the KERNEL'S OWN OSMetaClass registry, read by
 *               the inspector in bench/AccelDevice. NOT from disassembly: inside
 *               a collection the metaclass ctor is awkward to reach, and the
 *               registry is the authority our header must agree with anyway.
 *   superclass  IOService — same source (full chain:
 *               IOService -> IORegistryEntry -> OSObject).
 *   vtable      __ZTV13IOAccelerator, 270 slots, of which only the destructors
 *               and getMetaClass() are its own.
 *
 * WHY THIS ONE IS SAFE DESPITE COMING FROM A COLLECTION
 * A .kc-sourced class normally gets WEAKER verification: prelinked slots are
 * chained pointers whose cacheLevel-0 targets (the kernel's own methods) resolve
 * to nothing, so inherited slots read blank and verify_class.py can only compare
 * what Apple's side resolved. Here that does not matter, because the class adds
 * NO VIRTUALS AT ALL, and that was measured rather than assumed:
 *
 *     Apple's IOAccelerator vtable   270 slots   (vtable.py, --entry)
 *     clang's IOService vtable       269 entries (-fdump-vtable-layouts)
 *
 * vtable.py's count runs exactly one above clang's on every class checked
 * (IOAccelDevice2: 313 vs 312), so these are the SAME LENGTH. A subclass that
 * adds a virtual makes its vtable longer; this one does not. There is therefore
 * no slot ordering to get wrong — only the size, which the kernel confirms.
 */

#ifndef _IOACCELERATOR_H
#define _IOACCELERATOR_H

#include <IOKit/IOService.h>

/* From the kernel's OSMetaClass registry, not from disassembly. */
enum { kIOAcceleratorSize = 136 };

class IOAccelerator : public IOService
{
    OSDeclareDefaultStructors(IOAccelerator);

public:
    /*
     * The only three methods this class adds. NONE of them are virtual — they
     * are absent from the vtable — so their presence here cannot disturb slot
     * ordering. Parameter types are recovered from the mangled names:
     *
     *   __ZN13IOAccelerator13createAccelIDEjPi   (unsigned int, int *)
     *   __ZN13IOAccelerator13retainAccelIDEji    (unsigned int, int)
     *   __ZN13IOAccelerator14releaseAccelIDEji   (unsigned int, int)
     *
     * NOTE: the Itanium ABI mangles static and non-static member functions
     * identically, so the binary cannot tell us which these are. They are
     * declared static here to match the historical IOGraphics headers. This is
     * the one unverified detail in this file — and it is inert, because
     * declaring them generates no symbol reference unless they are CALLED.
     * If you ever call one, confirm the linkage first.
     */
    static IOReturn createAccelID(IOOptionBits options, int32_t *identifier);
    static IOReturn retainAccelID(IOOptionBits options, int32_t identifier);
    static IOReturn releaseAccelID(IOOptionBits options, int32_t identifier);

private:
    /*
     * Layout not recovered; total extent fixed by the registry size. The
     * compiler supplies sizeof(IOService) so this stays correct even if the
     * base changes between SDKs, and the static_assert catches it if not.
     */
    uint8_t _opaque[kIOAcceleratorSize - sizeof(IOService)];
};

static_assert(sizeof(IOAccelerator) == kIOAcceleratorSize,
              "IOAccelerator must be exactly 136 bytes — IOGraphicsAccelerator2 "
              "and every other subclass starts its own members at that offset "
              "and will corrupt the parent's ivars if this is wrong");

#endif /* _IOACCELERATOR_H */
