/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "IOAccelContext2.h"
#include "iofam_types.h"
enum { kIOAccel2DContext2Size = 0x1100 };
class IOAccel2DContext2 : public IOAccelContext2 {
    OSDeclareDefaultStructors(IOAccel2DContext2);
public:
    virtual void _pure362() = 0;
    virtual void _pure363() = 0;
    virtual void _RESERVEDIOAccel2DContext0();
    virtual void _RESERVEDIOAccel2DContext1();
    virtual void _RESERVEDIOAccel2DContext2();
    virtual void _RESERVEDIOAccel2DContext3();
    virtual void _RESERVEDIOAccel2DContext4();
    virtual void _RESERVEDIOAccel2DContext5();
private:
    uint8_t _opaque[kIOAccel2DContext2Size - sizeof(IOAccelContext2)];
};
static_assert(sizeof(IOAccel2DContext2) == kIOAccel2DContext2Size, "IOAccel2DContext2 must be 0x1100 bytes");
