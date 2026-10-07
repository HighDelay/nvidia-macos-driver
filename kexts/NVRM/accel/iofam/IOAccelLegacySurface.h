/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "IOAccelSurface.h"
#include "iofam_types.h"
enum { kIOAccelLegacySurfaceSize = 0x13b0 };
class IOAccelLegacySurface : public IOAccelSurface {
    OSDeclareDefaultStructors(IOAccelLegacySurface);
public:
    virtual void setSyncCommand(unsigned int*, unsigned int&, unsigned int&, _AMDSurfaceSwapSyncOptions const&) const = 0;
    virtual void didSubmitSwap(unsigned int, unsigned int);
    virtual bool isBackBufferReady(unsigned int);
    virtual void submitCopyForward(IOAccelEvent*, unsigned int, IOAccelResource2*, IOAccelResource2*, IOAccelBounds const*, unsigned int);
    virtual void submitUpdate(unsigned int, IOAccelBounds*, unsigned int);
    virtual void pickPresentType(unsigned int);
    virtual void _pure335() = 0;
private:
    uint8_t _opaque[kIOAccelLegacySurfaceSize - sizeof(IOAccelSurface)];
};
static_assert(sizeof(IOAccelLegacySurface) == kIOAccelLegacySurfaceSize, "IOAccelLegacySurface must be 0x13b0 bytes");
