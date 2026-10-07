/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include <IOKit/IOUserClient.h>
#include "iofam_types.h"
enum { kIOAccelSurfaceSize = 0x1338 };
class IOAccelSurface : public IOUserClient {
    OSDeclareDefaultStructors(IOAccelSurface);
public:
    virtual bool copyFromBuffer(int, int, int, int, unsigned int, unsigned int, IOAccelEvent*, IOAccelResource2*, IOAccelSysMemory*, unsigned int, unsigned long long, unsigned int) = 0;
    virtual bool copyToBuffer(int, int, int, int, unsigned int, unsigned int, IOAccelEvent*, IOAccelResource2*, IOAccelSysMemory*, unsigned int, unsigned long long, unsigned int) = 0;
    virtual void submitSwapCopy(IOAccelEvent*, IOAccelResource2*, IOAccelResource2*) = 0;
    virtual void surfaceStart();
    virtual void surfaceStop();
    virtual bool isSurfaceSizeSupported(short, short);
    virtual void shapeSurface(unsigned int, unsigned short, unsigned short) = 0;
    virtual uint32_t getDirtyBufferBitsFromPrivateModeBits(unsigned long long) = 0;
    virtual void orphanClientMappings(OSSet*);
    virtual void _RESERVEDIOAccelSurface0();
    virtual void _RESERVEDIOAccelSurface1();
    virtual void _RESERVEDIOAccelSurface2();
    virtual void _RESERVEDIOAccelSurface3();
    virtual void _RESERVEDIOAccelSurface4();
    virtual void _RESERVEDIOAccelSurface5();
    virtual void update_shape();
    virtual void prune_buffers();
    virtual void reset_req_bits();
    virtual void set_scaling(unsigned int, IOAccelSurfaceScaling*);
    virtual void add_context_to_list(IOAccelContext2*);
    virtual void remove_context_from_list(IOAccelContext2*);
    virtual void set_id_mode(unsigned int, unsigned int);
    virtual void set_shape(eIOAccelSurfaceShapeBits, unsigned int, IOAccelDeviceRegion*, unsigned long long);
    virtual void set_shape_backing_length_ext(eIOAccelSurfaceShapeBits, unsigned int, unsigned long long, unsigned int, unsigned long long, IOAccelDeviceRegion*, unsigned long long);
    virtual void surface_lock_options(eLockType, unsigned int, IOAccelSurfaceInformation*, unsigned long long);
    virtual void surface_unlock_options(eLockType, unsigned int);
    virtual void surface_control_with_lock(unsigned int, unsigned int, unsigned int*);
private:
    uint8_t _opaque[kIOAccelSurfaceSize - sizeof(IOUserClient)];
};
static_assert(sizeof(IOAccelSurface) == kIOAccelSurfaceSize, "IOAccelSurface must be 0x1338 bytes");
