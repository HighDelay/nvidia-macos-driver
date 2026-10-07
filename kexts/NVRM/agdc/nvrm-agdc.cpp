/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#include <IOKit/IOService.h>
#include <IOKit/IOLib.h>
#include <sys/sysctl.h>
#include <kern/thread_call.h>
#include <libkern/OSAtomic.h>
#define FBLOG(fmt, ...) IOLog("NVRM-agdc: " fmt "\n", ##__VA_ARGS__)
#include <mach/kmod.h>
extern "C" kern_return_t _start(kmod_info_t *ki, void *d); extern "C" kern_return_t _stop(kmod_info_t *ki, void *d);
extern "C" { KMOD_EXPLICIT_DECL(com.nullmoth.NVRMAGDC, "0.1", _start, _stop) }
extern "C" kern_return_t _start(kmod_info_t *ki, void *d) { return KERN_SUCCESS; }
extern "C" kern_return_t _stop(kmod_info_t *ki, void *d)  { return KERN_SUCCESS; }
#include "AppleGraphicsDeviceControl_re.h"
#include "nvrm-agdc-k5.h"
#pragma pack(push, 1)
struct NVAGDCVendorInfo {
    uint32_t version;
    char     vendor[0x20];
    uint32_t vendorID;
    uint32_t vendorClass;
};
struct NVAGDCGPUCapability {
    uint64_t portMask;
    uint64_t portFeatureA;
    uint64_t portFeatureB;
    uint64_t portFeatureC;
    uint32_t countA, countB, countC, countD;
    uint32_t maxFramebuffers;
    IOService *pciDevice;
    IOService *framebuffer[20];
};
#pragma pack(pop)
static_assert(sizeof(NVAGDCVendorInfo) == 0x2c, "AGDCVendorInfo_t is 0x2c");
static_assert(sizeof(NVAGDCGPUCapability) == 0xdc, "AGDCGPUCapability_t is 0xdc");

static int    gAgdcK5 = 0;
static SInt64 gAgdcK5Link = 0, gAgdcK5Caps = 0;
static int    gAgdcK5Why = 0;
static SInt64 gAgdcCmds = 0, gAgdcUnsupported = 0;
static int    gAgdcLastRefused = 0;
static int    gAgdcLogged = 0;

class NVDADeviceControl : public AppleGraphicsDeviceControl {
    OSDeclareDefaultStructors(NVDADeviceControl)
public:
    IOService *fPci; IOService *fFB;
    using AppleGraphicsDeviceControl::vendor_doDeviceAttribute;
    virtual IOReturn vendor_doDeviceAttribute(unsigned int cmd, unsigned long *in, unsigned long inSize,
                                              unsigned long *out, unsigned long *outSize, IOExternalMethodArguments *args) APPLE_KEXT_OVERRIDE;
};
OSDefineMetaClassAndStructors(NVDADeviceControl, AppleGraphicsDeviceControl)

static IOReturn nvdaK5Answer(IOService *fb, unsigned int cmd, unsigned long *in, unsigned long inSize, unsigned long *out, unsigned long *outSize)
{
    uint32_t t[NVK5_TIMING_WORDS]; unsigned long tBytes = 0; bool have = false;
    OSObject *o = fb ? fb->copyProperty("NVRMTiming") : NULL; OSData *d = OSDynamicCast(OSData, o);
    if (d) { have = true; tBytes = d->getLength(); bzero(t, sizeof t); if (tBytes == sizeof t) memcpy(t, d->getBytesNoCopy(), sizeof t); }
    OSSafeReleaseNULL(o);
    int why = 0;
    int r = nvk5_answer(cmd, in, inSize, out, outSize ? *outSize : 0, have ? t : NULL, tBytes, &why);
    if (r == NVK5_R_LINK) { OSAddAtomic64(1, &gAgdcK5Link); return kIOReturnSuccess; }
    if (r == NVK5_R_CAPS) { OSAddAtomic64(1, &gAgdcK5Caps); return kIOReturnSuccess; }
    if (r == NVK5_R_UNSUPPORTED) return kIOReturnUnsupported;
    if (r == NVK5_R_NOTFOUND && !fb) why = 0x30;
    gAgdcK5Why = why;
    if (gAgdcLogged < 40) { gAgdcLogged++; FBLOG("AGDC K5: command %#x NOT answered (why %#x) [%d/40]", cmd, why, gAgdcLogged); }
    return r == NVK5_R_BADARG ? kIOReturnBadArgument : kIOReturnNotFound;
}

IOReturn NVDADeviceControl::vendor_doDeviceAttribute(unsigned int cmd, unsigned long *in, unsigned long inSize,
                                                     unsigned long *out, unsigned long *outSize, IOExternalMethodArguments *args)
{
    OSAddAtomic64(1, &gAgdcCmds);
    if (cmd == 1) {
        if (!out || !outSize || *outSize != sizeof(NVAGDCVendorInfo)) return kIOReturnBadArgument;
        NVAGDCVendorInfo *v = (NVAGDCVendorInfo *)out; bzero(v, sizeof *v);
        v->version = 0x30000; strlcpy(v->vendor, "NVIDIA", sizeof v->vendor); v->vendorID = 0x10de; v->vendorClass = 2;
        return kIOReturnSuccess;
    }
    if (cmd == 0x980) {
        if (!out || !outSize || *outSize != sizeof(NVAGDCGPUCapability)) return kIOReturnBadArgument;
        NVAGDCGPUCapability *c = (NVAGDCGPUCapability *)out; bzero(c, sizeof *c);
        c->portMask = 1ull << 1;
        c->countA = c->countB = c->countC = c->countD = 1;
        c->maxFramebuffers = 1; c->pciDevice = fPci; c->framebuffer[0] = fFB;
        return kIOReturnSuccess;
    }
    if (gAgdcK5 && (cmd == NVK5_CMD_LINKCONFIG || cmd == NVK5_CMD_FBCAPEX)) {
        IOReturn k5 = nvdaK5Answer(fFB, cmd, in, inSize, out, outSize);
        if (k5 != kIOReturnUnsupported) return k5;
    }
    OSAddAtomic64(1, &gAgdcUnsupported); gAgdcLastRefused = (int)cmd;
    if (gAgdcLogged < 40) { gAgdcLogged++; FBLOG("AGDC: command %#x (in %lu bytes) -> Unsupported [%d/40]", cmd, inSize, gAgdcLogged); }
    return kIOReturnUnsupported;
}

static int gAgdcState = 0; static NVDADeviceControl *gAgdc = nullptr; static thread_call_t gAgdcCall = nullptr; static int gAgdcWant = 0;
static IOService *gAgdcFBRef = nullptr;
static void nvdaAgdcDropFB(void) { if (gAgdcFBRef) { gAgdcFBRef->release(); gAgdcFBRef = nullptr; } }
static void nvdaAgdcTargets(IOService **pci, IOService **fb)
{
    *pci = nullptr; *fb = nullptr;
    OSDictionary *m = IOService::serviceMatching("NVRMNVDAFramebuffer"); if (!m) return;
    IOService *f = IOService::copyMatchingService(m); m->release(); if (!f) return;
    IOService *p = f->getProvider(); int hops = 0;
    while (p && !p->metaCast("IOPCIDevice") && hops++ < 8) p = p->getProvider();
    if (!p || !p->metaCast("IOPCIDevice")) { f->release(); return; }
    gAgdcFBRef = f; *fb = f; *pci = p;
}

static SInt32 gAgdcBusy = 0;
static void nvdaAgdcWorkBody(void);
static void nvdaAgdcWork(thread_call_param_t, thread_call_param_t)
{
    OSIncrementAtomic(&gAgdcBusy); nvdaAgdcWorkBody(); OSDecrementAtomic(&gAgdcBusy);
}
static void nvdaAgdcWorkBody(void)
{
    if (gAgdcWant == 1 && !gAgdc) {
        IOService *pci = nullptr, *fb = nullptr; nvdaAgdcTargets(&pci, &fb);
        if (!pci || !fb) { gAgdcState = -1; FBLOG("AGDC: no PCI device or no framebuffer yet (pci %p fb %p)", pci, fb); return; }
        NVDADeviceControl *d = OSTypeAlloc(NVDADeviceControl);
        if (!d) { gAgdcState = -2; FBLOG("AGDC: alloc failed"); nvdaAgdcDropFB(); return; }
        if (!d->init(nullptr)) { gAgdcState = -3; FBLOG("AGDC: init failed"); d->release(); nvdaAgdcDropFB(); return; }
        d->fPci = pci; d->fFB = fb;
        if (!d->attach(pci)) { gAgdcState = -4; FBLOG("AGDC: attach(%s) failed", pci->getName()); d->release(); nvdaAgdcDropFB(); return; }
        if (!d->start(pci)) { gAgdcState = -5; FBLOG("AGDC: AppleGraphicsDeviceControl::start failed (vendor calls so far %lld)", gAgdcCmds); d->detach(pci); d->release(); nvdaAgdcDropFB(); return; }
        gAgdc = d; gAgdcState = 1;
        FBLOG("AGDC: UP on %s — vendor calls %lld, refused %lld (last %#x)", pci->getName(), gAgdcCmds, gAgdcUnsupported, gAgdcLastRefused);
    } else if (gAgdcWant == 0 && gAgdc) {
        NVDADeviceControl *d = gAgdc; gAgdc = nullptr;
        bool ok = d->terminate(kIOServiceRequired | kIOServiceSynchronous); d->release(); nvdaAgdcDropFB(); gAgdcState = 2;
        FBLOG("AGDC: terminate -> %d (a reboot is the certain way back; this is the polite one)", ok);
    }
}
static int nvrmfb_agdc_sysctl SYSCTL_HANDLER_ARGS
{
    int v = gAgdcState; int err = sysctl_handle_int(oidp, &v, 0, req);
    if (err || !req->newptr) return err;
    if (v != 0 && v != 1) return EINVAL;
    gAgdcWant = v;
    if (!gAgdcCall) gAgdcCall = thread_call_allocate(nvdaAgdcWork, nullptr);
    if (!gAgdcCall) return ENOMEM;
    thread_call_enter(gAgdcCall); return 0;
}
SYSCTL_PROC(_debug, OID_AUTO, nvrmfb_agdc, CTLTYPE_INT | CTLFLAG_RW | CTLFLAG_LOCKED, NULL, 0, nvrmfb_agdc_sysctl, "I",
            "write 1 = create our AppleGraphicsDeviceControl object, 0 = terminate it; reads the state (1 UP, <0 failed step)");
SYSCTL_QUAD(_debug, OID_AUTO, nvrmfb_agdc_cmds,    CTLFLAG_RD | CTLFLAG_LOCKED, &gAgdcCmds,        "AGDC vendor calls received");
SYSCTL_QUAD(_debug, OID_AUTO, nvrmfb_agdc_refused, CTLFLAG_RD | CTLFLAG_LOCKED, &gAgdcUnsupported, "AGDC vendor calls answered Unsupported");
SYSCTL_INT (_debug, OID_AUTO, nvrmfb_agdc_last_refused, CTLFLAG_RD | CTLFLAG_LOCKED, &gAgdcLastRefused, 0, "last AGDC command refused");
SYSCTL_INT (_debug, OID_AUTO, nvrmfb_agdc_k5,      CTLFLAG_RW | CTLFLAG_LOCKED, &gAgdcK5,     0, "K5: 1 = answer kAGDCLinkConfig + plane-scaler capabilities (0 after every boot)");
SYSCTL_QUAD(_debug, OID_AUTO, nvrmfb_agdc_k5_link, CTLFLAG_RD | CTLFLAG_LOCKED, &gAgdcK5Link,    "K5: kAGDCLinkConfig queries answered");
SYSCTL_QUAD(_debug, OID_AUTO, nvrmfb_agdc_k5_caps, CTLFLAG_RD | CTLFLAG_LOCKED, &gAgdcK5Caps,    "K5: plane-scaler capability queries answered");
SYSCTL_INT (_debug, OID_AUTO, nvrmfb_agdc_k5_why,  CTLFLAG_RD | CTLFLAG_LOCKED, &gAgdcK5Why,  0, "K5: why the last K5 query was not answered");
static void nvdaAgdcRemoveSysctl(void)
{
    sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc); sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc_cmds);
    sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc_refused); sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc_last_refused);
    sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc_k5); sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc_k5_link);
    sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc_k5_caps); sysctl_unregister_oid(&sysctl__debug_nvrmfb_agdc_k5_why);
}
static void nvdaAgdcInstallSysctl(void)
{
    sysctl_register_oid(&sysctl__debug_nvrmfb_agdc); sysctl_register_oid(&sysctl__debug_nvrmfb_agdc_cmds);
    sysctl_register_oid(&sysctl__debug_nvrmfb_agdc_refused); sysctl_register_oid(&sysctl__debug_nvrmfb_agdc_last_refused);
    sysctl_register_oid(&sysctl__debug_nvrmfb_agdc_k5); sysctl_register_oid(&sysctl__debug_nvrmfb_agdc_k5_link);
    sysctl_register_oid(&sysctl__debug_nvrmfb_agdc_k5_caps); sysctl_register_oid(&sysctl__debug_nvrmfb_agdc_k5_why);
}

class NVDAGDCLoader : public IOService {
    OSDeclareDefaultStructors(NVDAGDCLoader)
public:
    virtual bool start(IOService *provider) APPLE_KEXT_OVERRIDE;
    virtual void stop(IOService *provider) APPLE_KEXT_OVERRIDE;
};
OSDefineMetaClassAndStructors(NVDAGDCLoader, IOService)
bool NVDAGDCLoader::start(IOService *provider)
{
    if (!IOService::start(provider)) return false;
    nvdaAgdcInstallSysctl(); FBLOG("loaded; idle until `sysctl debug.nvrmfb_agdc=1`"); return true;
}
void NVDAGDCLoader::stop(IOService *provider)
{
    nvdaAgdcRemoveSysctl();
    if (gAgdcCall) {
        thread_call_cancel(gAgdcCall); int spins = 0;
        while (gAgdcBusy && spins++ < 200) IOSleep(10);
        if (gAgdcBusy) FBLOG("stop: the worker is STILL running after 2 s — leaving its thread call allocated rather than freeing it under it");
        else { thread_call_free(gAgdcCall); gAgdcCall = nullptr; }
    }
    if (gAgdc && !gAgdcBusy) { gAgdcWant = 0; nvdaAgdcWorkBody(); }
    IOService::stop(provider);
}
