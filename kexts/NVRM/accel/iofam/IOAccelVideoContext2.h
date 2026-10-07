/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "IOAccelContext2.h"
#include "iofam_types.h"
enum { kIOAccelVideoContext2Size = 0x10e8 };
class IOAccelVideoContext2 : public IOAccelContext2 {
    OSDeclareDefaultStructors(IOAccelVideoContext2);
public:
    virtual void _RESERVEDIOAccelVideoContext0();
    virtual void _RESERVEDIOAccelVideoContext1();
    virtual void _RESERVEDIOAccelVideoContext2();
    virtual void _RESERVEDIOAccelVideoContext3();
    virtual void _RESERVEDIOAccelVideoContext4();
    virtual void _RESERVEDIOAccelVideoContext5();
private:
    uint8_t _opaque[kIOAccelVideoContext2Size - sizeof(IOAccelContext2)];
};
static_assert(sizeof(IOAccelVideoContext2) == kIOAccelVideoContext2Size, "IOAccelVideoContext2 must be 0x10e8 bytes");
