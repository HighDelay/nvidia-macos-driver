/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include <libkern/c++/OSObject.h>
#include "iofam_types.h"
enum { kIOAccelShared2Size = 0x138 };
class IOAccelShared2 : public OSObject {
    OSDeclareDefaultStructors(IOAccelShared2);
public:
    virtual bool init(IOGraphicsAccelerator2*, task*);
    virtual void describeDriverAllocations(IOAccelAllocationInfo*);
    virtual void scrubEvents();
    virtual void orphanClientMappings(OSSet*);
    virtual void _RESERVEDIOAccelShared0();
    virtual void _RESERVEDIOAccelShared1();
    virtual void _RESERVEDIOAccelShared2();
    virtual void _RESERVEDIOAccelShared3();
    virtual void _RESERVEDIOAccelShared4();
    virtual void _RESERVEDIOAccelShared5();
private:
    uint8_t _opaque[kIOAccelShared2Size - sizeof(OSObject)];
};
static_assert(sizeof(IOAccelShared2) == kIOAccelShared2Size, "IOAccelShared2 must be 0x138 bytes");
