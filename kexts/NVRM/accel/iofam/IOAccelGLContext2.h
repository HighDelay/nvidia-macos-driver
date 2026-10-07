/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "IOAccelContext2.h"
#include "iofam_types.h"
enum { kIOAccelGLContext2Size = 0x1238 };
class IOAccelGLContext2 : public IOAccelContext2 {
    OSDeclareDefaultStructors(IOAccelGLContext2);
public:
    virtual void _pure362() = 0;
    virtual void _pure363() = 0;
    virtual void _pure364() = 0;
    virtual void sleepForSwapCompleteNoLock(unsigned int);
    virtual void addVendorSurfaceRequiredBits(unsigned long long);
    virtual void _pure367() = 0;
    virtual void processSwap(eDoSwap);
    virtual void _RESERVEDIOAccelGLContext0();
    virtual void _RESERVEDIOAccelGLContext1();
    virtual void _RESERVEDIOAccelGLContext2();
    virtual void _RESERVEDIOAccelGLContext3();
    virtual void _RESERVEDIOAccelGLContext4();
    virtual void _RESERVEDIOAccelGLContext5();
private:
    uint8_t _opaque[kIOAccelGLContext2Size - sizeof(IOAccelContext2)];
};
static_assert(sizeof(IOAccelGLContext2) == kIOAccelGLContext2Size, "IOAccelGLContext2 must be 0x1238 bytes");
