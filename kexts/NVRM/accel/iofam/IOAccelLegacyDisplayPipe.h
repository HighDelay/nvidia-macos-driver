/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "IOAccelDisplayPipe.h"
#include "iofam_types.h"
enum { kIOAccelLegacyDisplayPipeSize = 0x380 };
class IOAccelLegacyDisplayPipe : public IOAccelDisplayPipe {
    OSDeclareDefaultStructors(IOAccelLegacyDisplayPipe);
public:
    virtual bool initScanoutResource(unsigned int, IOAccelResource2*);
    virtual void destroyScanoutResource(unsigned int, IOAccelResource2*);
    virtual void setStereo(IOAccelStereoMode);
    virtual void needsSecondaryFramebufferResources();
    virtual bool isHardwareMirrorOfPipe(IOAccelLegacyDisplayPipe*);
    virtual void submitScanoutFlipBuffer(IOAccelEvent*, unsigned int, IOAccelResource2*, IOAccelResource2*);
    virtual void submitFlipBuffer(IOAccelEvent*, unsigned int, IOAccelResource2*, IOAccelResource2*);
    virtual void setupFullScreen(IOAccelResource2*, IOAccelResource2*);
    virtual void resetFullScreen(IOAccelEvent*, IOAccelResource2*, IOAccelResource2*);
    virtual void setupScanout(IOAccelResource2*, IOAccelResource2*);
    virtual void resetScanout(IOAccelEvent*, IOAccelResource2*, IOAccelResource2*);
    virtual bool isCurrentModeAcceleratorBacked();
private:
    uint8_t _opaque[kIOAccelLegacyDisplayPipeSize - sizeof(IOAccelDisplayPipe)];
};
static_assert(sizeof(IOAccelLegacyDisplayPipe) == kIOAccelLegacyDisplayPipeSize, "IOAccelLegacyDisplayPipe must be 0x380 bytes");
