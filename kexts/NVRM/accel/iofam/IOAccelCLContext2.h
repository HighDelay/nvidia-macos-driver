/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "IOAccelContext2.h"
#include "iofam_types.h"
enum { kIOAccelCLContext2Size = 0x10e8 };
class IOAccelCLContext2 : public IOAccelContext2 {
    OSDeclareDefaultStructors(IOAccelCLContext2);
public:
    virtual void _RESERVEDIOAccelCLContext0();
    virtual void _RESERVEDIOAccelCLContext1();
    virtual void _RESERVEDIOAccelCLContext2();
    virtual void _RESERVEDIOAccelCLContext3();
    virtual void _RESERVEDIOAccelCLContext4();
    virtual void _RESERVEDIOAccelCLContext5();
private:
    uint8_t _opaque[kIOAccelCLContext2Size - sizeof(IOAccelContext2)];
};
static_assert(sizeof(IOAccelCLContext2) == kIOAccelCLContext2Size, "IOAccelCLContext2 must be 0x10e8 bytes");
