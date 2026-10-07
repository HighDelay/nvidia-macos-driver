/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "iofam_types.h"
#include <libkern/c++/OSObject.h>
enum { kIOAccelTaskSize = 0x258 };
class IOAccelTask : public OSObject {
    OSDeclareDefaultStructors(IOAccelTask);
public:
    virtual bool init(IOGraphicsAccelerator2 *, unsigned int, IORangeAllocator **);
    virtual void freeToAllocGPUAddress(IOAccelMemoryMap *);
    virtual void freeAllGPUMappings();
    virtual void freeAllSysMemoryMappings();
    virtual void freeAllVidMemoryMappings();
    virtual unsigned long long allocate(IOAccelMemoryMap const *);
    virtual void deallocate(IOAccelMemoryMap const *, unsigned long long);
    virtual bool reserve(unsigned int, unsigned long long, unsigned long long);
    virtual void describeDriverAllocations(IOAccelAllocationInfo *);
    virtual void _RESERVEDIOAccelTask0();
    virtual void _RESERVEDIOAccelTask1();
    virtual void _RESERVEDIOAccelTask2();
    virtual void _RESERVEDIOAccelTask3();
    virtual void _RESERVEDIOAccelTask4();
    virtual void _RESERVEDIOAccelTask5();
    virtual void freeWaitToAllocGPUAddress(IOAccelMemoryMap *, bool);
private:
    uint8_t _opaque[kIOAccelTaskSize - sizeof(OSObject)];
};
static_assert(sizeof(IOAccelTask) == kIOAccelTaskSize, "IOAccelTask must be 600 bytes");
