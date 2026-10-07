/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "iofam_types.h"
#include "IOAccelDisplayMachine.h"
enum { kIOAccelLegacyDisplayMachineSize = 0x1f8 };
class IOAccelLegacyDisplayMachine : public IOAccelDisplayMachine {
    OSDeclareDefaultStructors(IOAccelLegacyDisplayMachine);
public:
    virtual bool start(IOPCIDevice *) APPLE_KEXT_OVERRIDE;
    virtual void stop() APPLE_KEXT_OVERRIDE;
    virtual void framebuffer_will_power_off(unsigned int) APPLE_KEXT_OVERRIDE;
    virtual void framebuffer_did_power_on(unsigned int) APPLE_KEXT_OVERRIDE;
    virtual void display_mode_will_change(unsigned int) APPLE_KEXT_OVERRIDE;
    virtual void display_mode_did_change(unsigned int) APPLE_KEXT_OVERRIDE;
    virtual void found_framebuffer(IOFramebuffer *) APPLE_KEXT_OVERRIDE;
    virtual bool initScanoutResource(unsigned int, unsigned int, IOFramebuffer *, IOAccelResource2 *);
    virtual void destroyScanoutResource(unsigned int, unsigned int, IOFramebuffer *, IOAccelResource2 *);
    virtual void setStereo(unsigned int, IOAccelStereoMode);
    virtual void setupFullScreen(unsigned int, IOAccelResource2 *, IOAccelResource2 *);
    virtual void resetFullScreen(IOAccelEvent *, unsigned int, IOAccelResource2 *, IOAccelResource2 *);
    virtual void submitFlipBuffer(IOAccelEvent *, unsigned int, unsigned int, IOAccelResource2 *, IOAccelResource2 *);
    virtual void setupScanout(unsigned int, IOAccelResource2 *, IOAccelResource2 *);
    virtual void resetScanout(IOAccelEvent *, unsigned int, IOAccelResource2 *, IOAccelResource2 *);
    virtual void submitScanoutFlipBuffer(IOAccelEvent *, unsigned int, unsigned int, IOAccelResource2 *, IOAccelResource2 *);
    virtual bool isFramebufferHardwareMirrorOfFramebuffer(unsigned int, unsigned int);
    virtual bool needsSecondaryFramebufferResources();
    virtual bool isCurrentFramebufferModeAcceleratorBacked(unsigned int);
    virtual void enableFlipInterrupt(unsigned int);
    virtual void disableFlipInterrupt(unsigned int);
private:
    uint8_t _opaque2[kIOAccelLegacyDisplayMachineSize - kIOAccelDisplayMachineSize];
};
static_assert(sizeof(IOAccelLegacyDisplayMachine) == kIOAccelLegacyDisplayMachineSize, "IOAccelLegacyDisplayMachine must be 504 bytes");
