/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "IOAccelMemory.h"
#include "iofam_types.h"
enum { kIOAccelVidMemorySize = 0x110 };
class IOAccelVidMemory : public IOAccelMemory {
    OSDeclareDefaultStructors(IOAccelVidMemory);
public:
    virtual uint64_t getDirtySize() const APPLE_KEXT_OVERRIDE;
    virtual uint64_t getResidentSize() const APPLE_KEXT_OVERRIDE;
    virtual void setPurgeable(unsigned int, unsigned int*) APPLE_KEXT_OVERRIDE;
    virtual uint64_t setTag(unsigned int) APPLE_KEXT_OVERRIDE;
    virtual uint64_t getTag() APPLE_KEXT_OVERRIDE;
    virtual bool matchForReuse(void*, unsigned long long) APPLE_KEXT_OVERRIDE;
    virtual void wire() APPLE_KEXT_OVERRIDE;
    virtual void unwire() APPLE_KEXT_OVERRIDE;
    virtual void increment_wire_count() APPLE_KEXT_OVERRIDE;
    virtual void decrement_wire_count() APPLE_KEXT_OVERRIDE;
    virtual bool init(IOGraphicsAccelerator2*, IOAccelShared2*, IOAccelResource2*, unsigned long long, void*);
    virtual bool allocPhysical() = 0;
    virtual void deallocPhysical() = 0;
    virtual void _RESERVEDIOAccelVidMemory0();
    virtual void _RESERVEDIOAccelVidMemory1();
    virtual void _RESERVEDIOAccelVidMemory2();
    virtual void _RESERVEDIOAccelVidMemory3();
    virtual void _RESERVEDIOAccelVidMemory4();
    virtual void _RESERVEDIOAccelVidMemory5();
private:
    uint8_t _opaque[kIOAccelVidMemorySize - sizeof(IOAccelMemory)];
};
static_assert(sizeof(IOAccelVidMemory) == kIOAccelVidMemorySize, "IOAccelVidMemory must be 0x110 bytes");
