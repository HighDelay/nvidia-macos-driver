/*
 * IOAccelSharedUserClient2 — reconstructed from IOAcceleratorFamily2
 * (macOS Sequoia, x86_64). Apple ships no header.
 *
 * This is the RESOURCE plane of the Metal driver: allocating GPU memory,
 * shared memory, fences and MTLEvents. Everything a Metal app does that isn't
 * command submission arrives through the 21 selectors below.
 *
 * All of it was READ OUT OF THE BINARY with ../re/vtable.py:
 *
 *   superclass   __ZN24IOAccelSharedUserClient210superClassE relocates against
 *                IOUserClient::gMetaClass                     -> IOUserClient
 *   class size   movl $0x168, %ecx in the metaclass ctor       -> 360 bytes
 *   vtable       __ZTV24IOAccelSharedUserClient2, 310 slots
 *   dispatch     __ZN24IOAccelSharedUserClient214sSharedMethodsE @ 0x77cb0,
 *                IOExternalMethod[21] — length proved by the next defined
 *                symbol landing at exactly table + 21*48.
 *
 * VTABLE ORDER DIFFERS FROM IOAccelDevice2. Here the six reserved slots come
 * FIRST (302..307) and the two new virtuals come AFTER them (308, 309). In
 * IOAccelDevice2 the new virtuals come first and the reserved slots trail. Do
 * not assume a house style — read each class's vtable. Getting this backwards
 * puts sharedStart() where a reserved slot belongs, which is a panic at
 * runtime and silence at build time.
 *
 * Verify any change with:  python3 re/verify_class.py IOAccelSharedUserClient2
 */

#ifndef _IOACCELSHAREDUSERCLIENT2_H
#define _IOACCELSHAREDUSERCLIENT2_H

#include <IOKit/IOUserClient.h>
#include <IOKit/IOService.h>

/*
 * Payload structs. The dispatch table gives exact SIZES but not field layouts,
 * so these are opaque and size-checked. Filling in invented fields would be a
 * guess dressed as a fact; decode them one at a time as they are needed.
 */
#ifndef IOACCEL_OPAQUE
#define IOACCEL_OPAQUE(name, bytes)                                            \
    struct name { uint8_t opaque[bytes]; };                                    \
    static_assert(sizeof(struct name) == (bytes), #name " wrong size")
#endif

IOACCEL_OPAQUE(IOAccelSharedPageoffResourceArgs,             8);
IOACCEL_OPAQUE(IOAccelDeviceShmemData,                      16);
IOACCEL_OPAQUE(IOAccelSharedGetInfoReturnData,              16);
IOACCEL_OPAQUE(IOAccelSharedSetupDirtyRingReturnData,       24);
IOACCEL_OPAQUE(IOAccelCreateMTLEventResult,                 24);
IOACCEL_OPAQUE(IOAccelMemoryData,                           48);
IOACCEL_OPAQUE(IOAccelAllocatedSize,                         8);
IOACCEL_OPAQUE(IOAccelResourceSetResourceOwnerIdentityData, 16);

/* Variable-sized across the boundary — no fixed size to assert. */
struct IOAccelNewResourceArgs;
struct IOAccelNewResourceReturnData;
struct IOAccelGetResourceInfoReturnData;

/* Argument of set_resource_purgeable; passed and returned by value as a scalar. */
typedef uint32_t eIOAccelResourcePurgeable;

enum { kIOAccelSharedUserClient2Size = 0x168 };

class IOAccelSharedUserClient2 : public IOUserClient
{
    OSDeclareDefaultStructors(IOAccelSharedUserClient2);

public:
    /* --- OSObject ------------------------------------------------ slot 20 */
    virtual void free() APPLE_KEXT_OVERRIDE;

    /* --- IOService -------------------------------- slots 129,131,186,187 */
    virtual bool requestTerminate(IOService *provider, IOOptionBits options)
        APPLE_KEXT_OVERRIDE;
    virtual bool didTerminate(IOService *provider, IOOptionBits options,
                              bool *defer) APPLE_KEXT_OVERRIDE;
    virtual bool start(IOService *provider) APPLE_KEXT_OVERRIDE;
    virtual void stop(IOService *provider) APPLE_KEXT_OVERRIDE;

    /* --- IOUserClient -------------------------- slots 268,288,293,298 */
    virtual IOReturn externalMethod(uint32_t selector,
                                    IOExternalMethodArguments *args,
                                    IOExternalMethodDispatch *dispatch,
                                    OSObject *target,
                                    void *reference) APPLE_KEXT_OVERRIDE;
    virtual IOReturn clientClose() APPLE_KEXT_OVERRIDE;
    virtual IOReturn connectClient(IOUserClient *client) APPLE_KEXT_OVERRIDE;
    virtual IOExternalMethod *getTargetAndMethodForIndex(IOService **targetP,
                                                         UInt32 index)
        APPLE_KEXT_OVERRIDE;

    /*
     * --- new virtuals ------------------------------------ slots 302..309
     * RESERVED SLOTS COME FIRST HERE. Opposite of IOAccelDevice2.
     * Base name is "IOAccelSharedUserClient" — no trailing 2 — because the
     * macro pastes className ## index and the binary exports
     * __ZN24IOAccelSharedUserClient223_RESERVEDIOAccelSharedUserClient0Ev.
     */
    OSMetaClassDeclareReservedUnused(IOAccelSharedUserClient, 0);  /* 302 */
    OSMetaClassDeclareReservedUnused(IOAccelSharedUserClient, 1);  /* 303 */
    OSMetaClassDeclareReservedUnused(IOAccelSharedUserClient, 2);  /* 304 */
    OSMetaClassDeclareReservedUnused(IOAccelSharedUserClient, 3);  /* 305 */
    OSMetaClassDeclareReservedUnused(IOAccelSharedUserClient, 4);  /* 306 */
    OSMetaClassDeclareReservedUnused(IOAccelSharedUserClient, 5);  /* 307 */

    virtual bool sharedStart();                                    /* 308 */
    virtual void sharedStop();                                     /* 309 */

public:
    /*
     * The 21 external methods. Non-virtual, so their order here is free; the
     * SELECTOR NUMBER is what the ABI fixes. Reached via
     * getTargetAndMethodForIndex -> sSharedMethods[selector].
     */
    IOReturn new_resource(IOAccelNewResourceArgs *in,
                          IOAccelNewResourceReturnData *out,
                          uint64_t inSize, uint32_t *outSize);        /* 0 */
    IOReturn delete_resource(uint32_t id);                            /* 1 */
    IOReturn page_off_resource(IOAccelSharedPageoffResourceArgs *a);  /* 2 */
    IOReturn finish_object_event(uint32_t a, uint32_t b);             /* 3 */
    IOReturn set_resource_purgeable(uint32_t id,
                                    eIOAccelResourcePurgeable state,
                                    eIOAccelResourcePurgeable *old);  /* 4 */
    IOReturn get_surface_info(uint32_t id, uint32_t *a, uint32_t *b,
                              uint32_t *c, uint32_t *d, uint32_t *e); /* 5 */
    IOReturn get_resource_info(uint32_t id,
                               IOAccelGetResourceInfoReturnData *out,
                               uint32_t *outSize);                    /* 6 */
    IOReturn create_shmem(uint32_t n, IOAccelDeviceShmemData *out);   /* 7 */
    IOReturn destroy_shmem(uint32_t id);                              /* 8 */
    IOReturn get_shared_info(IOAccelSharedGetInfoReturnData *out);    /* 9 */
    IOReturn setup_dirty_ring(
                 IOAccelSharedSetupDirtyRingReturnData *out);         /* 10 */
    IOReturn process_dirty_commands();                                /* 11 */
    IOReturn allocate_fence_memory(uint64_t *in, uint64_t *out);      /* 12 */
    IOReturn create_mtlevent(uint64_t *in,
                             IOAccelCreateMTLEventResult *out);       /* 13 */
    IOReturn destroy_mtlevent(uint32_t id);                           /* 14 */
    IOReturn get_memory_data(IOAccelMemoryData *out);                 /* 15 */
    IOReturn disconnect_peer(uint32_t id);                            /* 16 */
    IOReturn set_resources_purgeable(const uint32_t *ids,
                                     eIOAccelResourcePurgeable state,
                                     eIOAccelResourcePurgeable *old,
                                     int count);                      /* 17 */
    IOReturn get_resource_offset(uint64_t *in, uint64_t *out);        /* 18 */
    IOReturn get_allocated_size(IOAccelAllocatedSize *out);           /* 19 */
    IOReturn set_resource_owner_identity(
                 IOAccelResourceSetResourceOwnerIdentityData *in);    /* 20 */

    static IOExternalMethod sSharedMethods[21];

private:
    /* Layout not recovered; total extent fixed by the metaclass size. */
    uint8_t _opaque[kIOAccelSharedUserClient2Size - sizeof(IOUserClient)];
};

static_assert(sizeof(IOAccelSharedUserClient2) == kIOAccelSharedUserClient2Size,
              "IOAccelSharedUserClient2 must be exactly 0x168 bytes — a "
              "subclass's members start at that offset and will corrupt the "
              "parent's ivars if this is wrong");

/*
 * Selector map. Conventions and payload sizes decoded from sSharedMethods:
 *
 *   sel  convention       in            out
 *   0    StructIStructO   VARIABLE      VARIABLE
 *   1    ScalarIScalarO   1             0
 *   2    ScalarIStructI   0             8
 *   3    ScalarIScalarO   2             0
 *   4    ScalarIScalarO   2             1
 *   5    ScalarIScalarO   1             5
 *   6    ScalarIStructO   1             VARIABLE
 *   7    ScalarIStructO   1             16
 *   8    ScalarIScalarO   1             0
 *   9    ScalarIStructO   0             16
 *   10   ScalarIStructO   0             24
 *   11   ScalarIScalarO   0             0
 *   12   StructIStructO   8             8
 *   13   StructIStructO   8             24
 *   14   ScalarIScalarO   1             0
 *   15   ScalarIStructO   0             48
 *   16   ScalarIScalarO   1             0
 *   17   StructIStructO   VARIABLE      VARIABLE
 *   18   StructIStructO   16            8
 *   19   ScalarIStructO   0             8
 *   20   StructIStructO   16            0
 */
enum {
    kIOAccelSharedNewResource               = 0,
    kIOAccelSharedDeleteResource            = 1,
    kIOAccelSharedPageOffResource           = 2,
    kIOAccelSharedFinishObjectEvent         = 3,
    kIOAccelSharedSetResourcePurgeable      = 4,
    kIOAccelSharedGetSurfaceInfo            = 5,
    kIOAccelSharedGetResourceInfo           = 6,
    kIOAccelSharedCreateShmem               = 7,
    kIOAccelSharedDestroyShmem              = 8,
    kIOAccelSharedGetSharedInfo             = 9,
    kIOAccelSharedSetupDirtyRing            = 10,
    kIOAccelSharedProcessDirtyCommands      = 11,
    kIOAccelSharedAllocateFenceMemory       = 12,
    kIOAccelSharedCreateMTLEvent            = 13,
    kIOAccelSharedDestroyMTLEvent           = 14,
    kIOAccelSharedGetMemoryData             = 15,
    kIOAccelSharedDisconnectPeer            = 16,
    kIOAccelSharedSetResourcesPurgeable     = 17,
    kIOAccelSharedGetResourceOffset         = 18,
    kIOAccelSharedGetAllocatedSize          = 19,
    kIOAccelSharedSetResourceOwnerIdentity  = 20,
    kIOAccelSharedMethodCount               = 21,
};

#endif /* _IOACCELSHAREDUSERCLIENT2_H */
