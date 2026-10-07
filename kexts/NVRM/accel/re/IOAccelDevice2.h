/*
 * IOAccelDevice2 — reconstructed from IOAcceleratorFamily2 (macOS Sequoia, x86_64).
 *
 * Apple ships no header for this class. Everything below was read out of the
 * binary with ../re/vtable.py, not inferred from documentation or guessed:
 *
 *   superclass    __ZN14IOAccelDevice210superClassE relocates against
 *                 IOUserClient::gMetaClass                        -> IOUserClient
 *   class size    OSMetaClass ctor immediate in
 *                 __ZN14IOAccelDevice29MetaClassC2Ev: movl $0x188 -> 392 bytes
 *   vtable        __ZTV14IOAccelDevice2 @ 0x6f100, 311 slots
 *   dispatch      __ZN14IOAccelDevice214sDeviceMethodsE @ 0x6fbb0,
 *                 legacy IOExternalMethod[10], 48-byte stride
 *
 * THE TWO THINGS THAT MUST NOT DRIFT
 *
 * 1. VTABLE ORDER. The three new virtuals occupy slots 302/303/304 and the six
 *    reserved slots follow at 305..310. Declaring them in any other order, or
 *    omitting a reserved slot, moves every later entry. A virtual call then
 *    lands on the wrong function — a panic at runtime, silence at compile time.
 *
 * 2. CLASS SIZE. A subclass's own members start at offset 392. If this header
 *    computes a smaller size, those members overlap IOAccelDevice2's private
 *    ivars and corrupt them. Nothing warns. The static_assert below is the
 *    guard; if it ever fires, re-read the metaclass ctor rather than adjusting
 *    the number to make it pass.
 */

#ifndef _IOACCELDEVICE2_H
#define _IOACCELDEVICE2_H

#include <IOKit/IOUserClient.h>
#include <IOKit/IOService.h>

/*
 * Payload structs. The dispatch table gives their exact SIZES; it does not give
 * their field layouts. They are therefore declared opaque and size-checked —
 * inventing plausible fields would be a guess wearing the costume of a fact.
 * Fill them in only as each one is actually decoded.
 */
#define IOACCEL_OPAQUE(name, bytes)                                            \
    struct name { uint8_t opaque[bytes]; };                                    \
    static_assert(sizeof(struct name) == (bytes), #name " wrong size")

IOACCEL_OPAQUE(IOAccelDeviceConfigData,        64);
IOACCEL_OPAQUE(IOAccelDeviceEventMachineData, 600);
IOACCEL_OPAQUE(IOAccelDeviceSurfaceData,       24);
IOACCEL_OPAQUE(IOAccelDeviceGlobalObjectIDData, 8);
IOACCEL_OPAQUE(IOAccelDeviceTraceFilterData,    8);
IOACCEL_OPAQUE(IOAccelDeviceInfoReturnData,    24);
IOACCEL_OPAQUE(IOAccelDeviceGIDGroupData,      16);
IOACCEL_OPAQUE(IOAccelDeviceAPIProperty,       16);

/* get_name() writes into a caller-supplied buffer of exactly this size. */
enum { kIOAccelDeviceNameSize = 64 };

/* Total object size from the metaclass constructor. */
enum { kIOAccelDevice2Size = 0x188 };

class IOAccelDevice2 : public IOUserClient
{
    OSDeclareDefaultStructors(IOAccelDevice2);

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

    /* --- IOUserClient ----------------------------- slots 268,288,298 */
    virtual IOReturn externalMethod(uint32_t selector,
                                    IOExternalMethodArguments *args,
                                    IOExternalMethodDispatch *dispatch,
                                    OSObject *target,
                                    void *reference) APPLE_KEXT_OVERRIDE;
    virtual IOReturn clientClose() APPLE_KEXT_OVERRIDE;
    virtual IOExternalMethod *getTargetAndMethodForIndex(IOService **targetP,
                                                         UInt32 index)
        APPLE_KEXT_OVERRIDE;

    /*
     * --- IOAccelDevice2's own virtuals -------------- slots 302,303,304
     * ORDER IS LOAD-BEARING. Do not reorder, do not insert.
     */
    virtual bool deviceStart();                       /* slot 302 */
    virtual void deviceStop();                        /* slot 303 */
    virtual void orphanClientMappings(OSSet *set);    /* slot 304 */

    /*
     * Reserved padding, slots 305..310. Six, exactly.
     *
     * The argument is "IOAccelDevice", NOT "IOAccelDevice2". The macro pastes
     * className ## index, and the shipping binary exports
     * __ZN14IOAccelDevice223_RESERVEDIOAccelDevice0Ev — i.e. the method is
     * named _RESERVEDIOAccelDevice0. Passing the class's own name yields
     * _RESERVEDIOAccelDevice20, a symbol that exists nowhere, and the kext
     * fails to link with "could not find a kext which exports this symbol".
     * The vtable ORDER is identical either way, so nothing catches this except
     * comparing the emitted names against the binary.
     */
    OSMetaClassDeclareReservedUnused(IOAccelDevice, 0);
    OSMetaClassDeclareReservedUnused(IOAccelDevice, 1);
    OSMetaClassDeclareReservedUnused(IOAccelDevice, 2);
    OSMetaClassDeclareReservedUnused(IOAccelDevice, 3);
    OSMetaClassDeclareReservedUnused(IOAccelDevice, 4);
    OSMetaClassDeclareReservedUnused(IOAccelDevice, 5);

public:
    /*
     * Non-virtual members. These are absent from the vtable, so their order is
     * irrelevant to binary compatibility; they are the targets of the ten
     * entries in sDeviceMethods, reached via getTargetAndMethodForIndex.
     */
    bool init(OSDictionary *properties, task_t owningTask);

    IOReturn get_config(IOAccelDeviceConfigData *out);              /* sel 0 */
    IOReturn get_name(char *out);                                   /* sel 1 */
    IOReturn get_event_machine(IOAccelDeviceEventMachineData *out); /* sel 2 */
    IOReturn get_surface_info(uint32_t id,
                              IOAccelDeviceSurfaceData *out);       /* sel 3 */
    IOReturn set_stereo(uint32_t a, uint32_t b);                    /* sel 4 */
    IOReturn get_next_global_object_id(
                 IOAccelDeviceGlobalObjectIDData *out);             /* sel 5 */
    IOReturn get_current_trace_filter(
                 IOAccelDeviceTraceFilterData *out);                /* sel 6 */
    IOReturn get_device_info(IOAccelDeviceInfoReturnData *out);     /* sel 7 */
    IOReturn get_next_gid_group(IOAccelDeviceGIDGroupData *out);    /* sel 8 */
    IOReturn set_api_property(IOAccelDeviceAPIProperty *inout);     /* sel 9 */

    void stopLocked();
    void detach_device();
    void add_orphaned_mappings(OSSet *set);

    static IOExternalMethod sDeviceMethods[10];

private:
    /*
     * IOAccelDevice2's private ivars, whose layout is not recovered. Their
     * total extent is fixed by the metaclass size; the compiler supplies
     * sizeof(IOUserClient) so this stays correct across OS revisions where the
     * base class changes but 0x188 does not.
     */
    uint8_t _opaque[kIOAccelDevice2Size - sizeof(IOUserClient)];
};

static_assert(sizeof(IOAccelDevice2) == kIOAccelDevice2Size,
              "IOAccelDevice2 must be exactly 0x188 bytes — a subclass's "
              "members start at that offset and will corrupt the parent's "
              "ivars if this is wrong");

/*
 * Selector numbers for sDeviceMethods, from the decoded table.
 * Calling convention and payload size per selector:
 *
 *   sel  convention       scalars in   struct out
 *   0    ScalarIStructO   0            64
 *   1    ScalarIStructO   0            64
 *   2    ScalarIStructO   0            600
 *   3    ScalarIStructO   1            24
 *   4    ScalarIScalarO   2            0
 *   5    ScalarIStructO   0            8
 *   6    ScalarIStructO   0            8
 *   7    ScalarIStructO   0            24
 *   8    ScalarIStructO   0            16
 *   9    StructIStructO   16 in        variable
 */
enum {
    kIOAccelDeviceGetConfig             = 0,
    kIOAccelDeviceGetName               = 1,
    kIOAccelDeviceGetEventMachine       = 2,
    kIOAccelDeviceGetSurfaceInfo        = 3,
    kIOAccelDeviceSetStereo             = 4,
    kIOAccelDeviceGetNextGlobalObjectID = 5,
    kIOAccelDeviceGetCurrentTraceFilter = 6,
    kIOAccelDeviceGetDeviceInfo         = 7,
    kIOAccelDeviceGetNextGIDGroup       = 8,
    kIOAccelDeviceSetAPIProperty        = 9,
    kIOAccelDeviceMethodCount           = 10,
};

#endif /* _IOACCELDEVICE2_H */
