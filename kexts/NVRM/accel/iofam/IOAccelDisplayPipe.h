/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#pragma once
#include <IOKit/IOService.h>
#include "iofam_types.h"
enum { kIOAccelDisplayPipeSize = 0x318 };
class IOAccelDisplayPipe : public IOService {
    OSDeclareDefaultStructors(IOAccelDisplayPipe);
public:
    virtual bool init(IOGraphicsAccelerator2*, IOAccelDisplayMachine*, IOFramebuffer*, unsigned int);
    virtual IOAccelMemory *initFramebufferResource(unsigned int, IOAccelResource2*);
    virtual void destroyFramebufferResource(unsigned int, IOAccelResource2*);
    virtual void displayModeWillChange();
    virtual void displayModeDidChange();
    virtual void enableVBLInterrupt();
    virtual void disableVBLInterrupt();
    virtual void enableTransactionInterrupt();
    virtual void disableTransactionInterrupt();
    virtual OSObject * newDisplayPipeTransaction();
    virtual IOReturn validateTransaction(IOAccelDisplayPipeTransaction2*);
    virtual IOReturn performTransaction(IOAccelDisplayPipeTransaction2*);
    virtual bool isTransactionComplete(IOAccelDisplayPipeTransaction2*);
    virtual void submitTransaction(IOAccelDisplayPipeTransaction2*);
    virtual void getDisplayModePipeScalerSetup(IOAccelDisplayPipeScaler*);
    virtual OSObject * createWorkLoop();
    virtual void copyCapabilities();
    virtual void beginTransaction(IOAccelEvent*);
    virtual void signalTransactionComplete(IOAccelEvent*);
    virtual void framebufferTerminated();
    virtual void wsaaEnterDefer(int);
    virtual void wsaaWillExitDefer(int);
    virtual void wsaaDidExitDefer(int);
    virtual void wsaaWillEnterDefer(int);
    virtual void wsaaDidEnterDefer(int);
    virtual void willPowerOff();
    virtual void didPowerOn();
    virtual void logTransactionTimeoutDiagnosisReport();
    virtual void getStampIndex() const;
    virtual void triage(char**, unsigned long long*);
    virtual void _RESERVEDIOAccelDisplayPipe1();
    virtual void _RESERVEDIOAccelDisplayPipe2();
    virtual void _RESERVEDIOAccelDisplayPipe3();
    virtual void _RESERVEDIOAccelDisplayPipe4();
    virtual void _RESERVEDIOAccelDisplayPipe5();
    virtual void _RESERVEDIOAccelDisplayPipe6();
    virtual void _RESERVEDIOAccelDisplayPipe7();
    virtual void framebuffer_will_power_off();
    virtual void framebuffer_did_power_on();
    virtual void framebuffer_terminated();
private:
    uint8_t _opaque[kIOAccelDisplayPipeSize - sizeof(IOService)];
};
static_assert(sizeof(IOAccelDisplayPipe) == kIOAccelDisplayPipeSize, "IOAccelDisplayPipe must be 0x318 bytes");
