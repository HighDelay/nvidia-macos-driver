/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include "iofam_types.h"
#include <IOKit/IOService.h>
enum { kIOAccelDisplayMachineSize = 0x178 };
class IOAccelDisplayMachine : public IOService {
    OSDeclareDefaultStructors(IOAccelDisplayMachine);
public:
    virtual bool init(IOGraphicsAccelerator2 *);
    virtual bool start(IOPCIDevice *);
    virtual void stop();
    virtual bool displayModeWillChange() = 0;
    virtual bool displayModeDidChange() = 0;
    virtual bool initFramebufferResource(unsigned int, unsigned int, IOFramebuffer *, IOAccelResource2 *);
    virtual void destroyFramebufferResource(unsigned int, unsigned int, IOFramebuffer *, IOAccelResource2 *);
    virtual void triage(char **, unsigned long long *);
    virtual void _RESERVEDIOAccelDisplayMachine1();
    virtual void _RESERVEDIOAccelDisplayMachine2();
    virtual void _RESERVEDIOAccelDisplayMachine3();
    virtual void _RESERVEDIOAccelDisplayMachine4();
    virtual void _RESERVEDIOAccelDisplayMachine5();
    virtual void framebuffer_will_power_off(unsigned int);
    virtual void framebuffer_did_power_on(unsigned int);
    virtual void display_mode_will_change(unsigned int);
    virtual void display_mode_did_change(unsigned int);
    virtual void found_framebuffer(IOFramebuffer *);

    unsigned int getFramebufferCount() const;
private:
    uint8_t _opaque[kIOAccelDisplayMachineSize - sizeof(IOService)];
};
static_assert(sizeof(IOAccelDisplayMachine) == kIOAccelDisplayMachineSize, "IOAccelDisplayMachine must be 376 bytes");
