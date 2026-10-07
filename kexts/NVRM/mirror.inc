/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#import <Foundation/Foundation.h>
#import <CoreGraphics/CoreGraphics.h>
#import <IOSurface/IOSurface.h>
#import <dlfcn.h>

typedef void *CGDSRef; typedef int32_t CGDSStatus; typedef const void *CGDSUpdateRef;
typedef void (^CGDSHandler)(CGDSStatus status, uint64_t displayTime, IOSurfaceRef frame, CGDSUpdateRef update);
typedef CGDSRef (*cgds_create_t)(CGDirectDisplayID, size_t, size_t, int32_t, CFDictionaryRef, dispatch_queue_t, CGDSHandler);
typedef CGError (*cgds_startstop_t)(CGDSRef);
enum { kCGDSFrameComplete = 0 };
@interface CGVirtualDisplayDescriptor : NSObject
@property (nonatomic, strong) dispatch_queue_t queue; @property (nonatomic, copy) NSString *name;
@property (nonatomic) uint32_t maxPixelsWide, maxPixelsHigh; @property (nonatomic) CGSize sizeInMillimeters;
@property (nonatomic) uint32_t vendorID, productID, serialNum; @property (nonatomic, copy) void (^terminationHandler)(id, id);
@end
@interface CGVirtualDisplayMode : NSObject
- (instancetype)initWithWidth:(uint32_t)w height:(uint32_t)h refreshRate:(double)r;
@end
@interface CGVirtualDisplaySettings : NSObject
@property (nonatomic) uint32_t hiDPI; @property (nonatomic, strong) NSArray *modes;
@end
@interface CGVirtualDisplay : NSObject
- (instancetype)initWithDescriptor:(CGVirtualDisplayDescriptor *)d; - (BOOL)applySettings:(CGVirtualDisplaySettings *)s;
@property (readonly) uint32_t displayID;
@end

static int cmd_kms_mirror(unsigned seconds)
{
    io_connect_t ctl = nv_open(0), dev = nv_open(1);
    nv_ioctl_card_info_t ci; memset(&ci, 0, sizeof ci);
    if (nv_esc(ctl, NV_ESC_CARD_INFO, &ci, sizeof ci) != 0 || !ci.valid) { printf("no card info\n"); return 1; }
    nv_ioctl_register_fd_t rf; rf.ctl_fd = nv_fd(ctl); nv_esc(dev, NV_ESC_REGISTER_FD, &rf, sizeof rf);
    NvHandle hClient = rm_alloc(ctl, 0, 0, 0, NV01_ROOT_CLIENT_CLASS, NULL, 0, "NV01_ROOT_CLIENT"); if (!hClient) return 1;
    NV0000_CTRL_GPU_ATTACH_IDS_PARAMS at; memset(&at, 0, sizeof at); at.gpuIds[0] = ci.gpu_id; at.gpuIds[1] = NV0000_CTRL_GPU_INVALID_ID;
    rm_control(ctl, hClient, hClient, NV0000_CTRL_CMD_GPU_ATTACH_IDS, &at, sizeof at);
    NV0080_ALLOC_PARAMETERS dp; memset(&dp, 0, sizeof dp); dp.deviceId = 0; dp.hClientShare = hClient;
    NvHandle hDevice = rm_alloc(ctl, hClient, hClient, 0xcafe0001, NV01_DEVICE_0, &dp, sizeof dp, "NV01_DEVICE_0"); if (!hDevice) return 1;
    NV2080_ALLOC_PARAMETERS sp; memset(&sp, 0, sizeof sp);
    NvHandle hSub = rm_alloc(ctl, hClient, hDevice, 0xcafe0002, NV20_SUBDEVICE_0, &sp, sizeof sp, "NV20_SUBDEVICE_0"); if (!hSub) return 1;
    struct NvKmsAllocDeviceParams ad; memset(&ad, 0, sizeof ad);
    strncpy(ad.request.versionString, NV_VERSION_STRING, sizeof ad.request.versionString - 1); ad.request.deviceId.rmDeviceId = 0;
    if (kms_ioctl(dev, NVKMS_IOCTL_ALLOC_DEVICE, &ad, sizeof ad, "ALLOC_DEVICE") || ad.reply.status != NVKMS_ALLOC_DEVICE_STATUS_SUCCESS) return 1;
    NvKmsDeviceHandle hDev = ad.reply.deviceHandle; NvKmsDispHandle hDisp = ad.reply.dispHandles[0];
    struct NvKmsQueryDispParams qd; memset(&qd, 0, sizeof qd); qd.request.deviceHandle = hDev; qd.request.dispHandle = hDisp;
    if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_DISP, &qd, sizeof qd, "QUERY_DISP")) return 1;
    NVDpyId dpy = nvInvalidDpyId(); NvU32 headMask = 0; char dpyName[NVKMS_DPY_NAME_SIZE] = "display"; unsigned cmW = 29, cmH = 17;
    for (unsigned bit = 0; bit < 32 && nvDpyIdIsInvalid(dpy); bit++) {
        if (!(nvDpyIdListToNvU32(qd.reply.validDpys) & (1u << bit))) continue;
        struct NvKmsQueryDpyDynamicDataParams *qy = calloc(1, sizeof *qy);
        qy->request.deviceHandle = hDev; qy->request.dispHandle = hDisp; qy->request.dpyId = nvNvU32ToDpyId(1u << bit);
        if (!kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_DYNAMIC_DATA, qy, sizeof *qy, "QUERY_DPY_DYNAMIC_DATA") && qy->reply.connected) {
            dpy = qy->request.dpyId; strncpy(dpyName, qy->reply.name, sizeof dpyName - 1);
            if (qy->reply.physicalDimensions.widthInCM) { cmW = qy->reply.physicalDimensions.widthInCM; cmH = qy->reply.physicalDimensions.heightInCM; }
            struct NvKmsQueryDpyStaticDataParams qs; memset(&qs, 0, sizeof qs);
            qs.request.deviceHandle = hDev; qs.request.dispHandle = hDisp; qs.request.dpyId = dpy;
            if (!kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_STATIC_DATA, &qs, sizeof qs, "QUERY_DPY_STATIC_DATA")) headMask = qs.reply.headMask;
        }
        free(qy);
    }
    if (nvDpyIdIsInvalid(dpy)) { printf("no connected display\n"); return 1; }
    NvU32 head = 0; while (head < 32 && !(headMask & (1u << head))) head++;
    struct NvKmsValidateModeIndexParams *vm = calloc(1, sizeof *vm); struct NvKmsMode mode; int haveMode = 0;
    for (NvU32 i = 0; i < 256; i++) {
        memset(vm, 0, sizeof *vm); vm->request.deviceHandle = hDev; vm->request.dispHandle = hDisp; vm->request.dpyId = dpy; vm->request.modeIndex = i;
        if (kms_ioctl(dev, NVKMS_IOCTL_VALIDATE_MODE_INDEX, vm, sizeof *vm, "VALIDATE_MODE_INDEX") || vm->reply.end) break;
        if (vm->reply.valid && (!haveMode || vm->reply.preferredMode)) { mode = vm->reply.mode; haveMode = 1; if (vm->reply.preferredMode) break; }
    }
    free(vm); if (!haveMode) { printf("no valid mode\n"); return 1; }
    unsigned w = mode.timings.hVisible, h = mode.timings.vVisible; unsigned pitch = (w * 4 + 255) & ~255u; NvU64 size = ((NvU64)pitch * h + 0xffff) & ~0xffffull;
    NV_MEMORY_ALLOCATION_PARAMS mp; memset(&mp, 0, sizeof mp);
    mp.owner = hClient; mp.type = NVOS32_TYPE_IMAGE; mp.size = size; mp.alignment = 65536;
    mp.attr = DRF_DEF(OS32, _ATTR, _PAGE_SIZE, _4KB) | DRF_DEF(OS32, _ATTR, _LOCATION, _VIDMEM) | DRF_DEF(OS32, _ATTR, _COHERENCY, _WRITE_COMBINE) | DRF_DEF(OS32, _ATTR, _FORMAT, _PITCH);
    mp.flags = NVOS32_ALLOC_FLAGS_ALIGNMENT_FORCE;
    NvHandle hMem = rm_alloc(ctl, hClient, hDevice, 0xcafe0031, NV01_MEMORY_LOCAL_USER, &mp, sizeof mp, "NV01_MEMORY_LOCAL_USER (surface)"); if (!hMem) return 1;
    io_connect_t mc = nv_open(1);
    nv_ioctl_nvos33_parameters_with_fd m; memset(&m, 0, sizeof m);
    m.params.hClient = hClient; m.params.hDevice = hSub; m.params.hMemory = hMem; m.params.length = size; m.fd = nv_fd(mc);
    if (nv_esc(ctl, NV_ESC_RM_MAP_MEMORY, &m, sizeof m) != 0 || m.params.status != 0) { printf("map surface failed 0x%x\n", m.params.status); return 1; }
    mach_vm_address_t addr = 0; mach_vm_size_t len = 0;
    if (IOConnectMapMemory64(mc, 0, mach_task_self(), &addr, &len, kIOMapAnywhere) != KERN_SUCCESS) { printf("IOConnectMapMemory64 failed\n"); return 1; }
    volatile uint32_t *px = (volatile uint32_t *)(uintptr_t)addr;
    for (NvU64 i = 0; i < size / 4; i++) px[i] = 0x00202020;
    struct NvKmsRegisterSurfaceParams rs; memset(&rs, 0, sizeof rs);
    rs.request.deviceHandle = hDev; rs.request.rmClient = hClient; rs.request.planes[0].u.rmObject = hMem; rs.request.planes[0].pitch = pitch; rs.request.planes[0].rmObjectSizeInBytes = size;
    rs.request.widthInPixels = w; rs.request.heightInPixels = h; rs.request.layout = NvKmsSurfaceMemoryLayoutPitch; rs.request.format = NvKmsSurfaceMemoryFormatX8R8G8B8; rs.request.isoType = NVKMS_MEMORY_ISO;
    if (kms_ioctl(dev, NVKMS_IOCTL_REGISTER_SURFACE, &rs, sizeof rs, "REGISTER_SURFACE") || !rs.reply.surfaceHandle) return 1;
    struct NvKmsGrabOwnershipParams go; memset(&go, 0, sizeof go); go.request.deviceHandle = hDev;
    if (kms_ioctl(dev, NVKMS_IOCTL_GRAB_OWNERSHIP, &go, sizeof go, "GRAB_OWNERSHIP")) return 1;
    struct NvKmsSetModeParams *sm = calloc(1, sizeof *sm);
    sm->request.deviceHandle = hDev; sm->request.commit = NV_TRUE; sm->request.requestedDispsBitMask = 1; sm->request.disp[0].requestedHeadsBitMask = 1u << head;
    struct NvKmsSetModeOneHeadRequest *hr = &sm->request.disp[0].head[head];
    hr->dpyIdList = nvAddDpyIdToEmptyDpyIdList(dpy); hr->mode = mode; hr->viewPortSizeIn.width = w; hr->viewPortSizeIn.height = h;
    hr->viewPortOut.width = w; hr->viewPortOut.height = h; hr->viewPortOutSpecified = NV_TRUE;
    hr->flip.layer[NVKMS_MAIN_LAYER].surface.specified = NV_TRUE; hr->flip.layer[NVKMS_MAIN_LAYER].surface.handle[NVKMS_LEFT] = rs.reply.surfaceHandle;
    hr->flip.layer[NVKMS_MAIN_LAYER].sizeIn.specified = NV_TRUE; hr->flip.layer[NVKMS_MAIN_LAYER].sizeIn.val.width = w; hr->flip.layer[NVKMS_MAIN_LAYER].sizeIn.val.height = h;
    hr->flip.layer[NVKMS_MAIN_LAYER].sizeOut.specified = NV_TRUE; hr->flip.layer[NVKMS_MAIN_LAYER].sizeOut.val.width = w; hr->flip.layer[NVKMS_MAIN_LAYER].sizeOut.val.height = h;
    if (kms_ioctl(dev, NVKMS_IOCTL_SET_MODE, sm, sizeof *sm, "SET_MODE") || sm->reply.status != NVKMS_SET_MODE_STATUS_SUCCESS) { printf("SET_MODE failed\n"); return 1; }
    printf("-- modeset: %s %ux%u on head %u, surface at 0x%llx (pitch %u)\n", dpyName, w, h, head, (unsigned long long)addr, pitch);

    @autoreleasepool {
        dispatch_queue_t q = dispatch_queue_create("nvrm.mirror", DISPATCH_QUEUE_SERIAL);
        CGVirtualDisplayDescriptor *d = [[NSClassFromString(@"CGVirtualDisplayDescriptor") alloc] init];
        d.queue = q; d.name = [NSString stringWithFormat:@"NVIDIA RTX 5060 — %s", dpyName];
        d.maxPixelsWide = w; d.maxPixelsHigh = h; d.sizeInMillimeters = CGSizeMake(cmW * 10, cmH * 10);
        d.vendorID = 0x10de; d.productID = 0x2d05; d.serialNum = 5060;
        d.terminationHandler = ^(id a, id b) { printf("virtual display terminated\n"); };
        CGVirtualDisplay *vd = [[NSClassFromString(@"CGVirtualDisplay") alloc] initWithDescriptor:d];
        if (!vd) { printf("CGVirtualDisplay init failed\n"); return 1; }
        CGVirtualDisplaySettings *s = [[NSClassFromString(@"CGVirtualDisplaySettings") alloc] init];
        s.hiDPI = 0; s.modes = @[[[NSClassFromString(@"CGVirtualDisplayMode") alloc] initWithWidth:w height:h refreshRate:60.0]];
        BOOL ok = [vd applySettings:s];
        CGDirectDisplayID did = vd.displayID;
        printf("-- virtual display: applySettings %d, displayID %u, bounds %.0fx%.0f, online %d\n", ok, did, CGDisplayBounds(did).size.width, CGDisplayBounds(did).size.height, CGDisplayIsOnline(did));
        if (!did) return 1;

        __block unsigned long frames = 0, bytes = 0;
        void *cg = dlopen("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics", RTLD_NOW);
        cgds_create_t dsCreate = (cgds_create_t)dlsym(cg, "CGDisplayStreamCreateWithDispatchQueue");
        cgds_startstop_t dsStart = (cgds_startstop_t)dlsym(cg, "CGDisplayStreamStart"), dsStop = (cgds_startstop_t)dlsym(cg, "CGDisplayStreamStop");
        if (!dsCreate || !dsStart || !dsStop) { printf("CGDisplayStream entry points not in CoreGraphics (%p %p %p) -> ScreenCaptureKit needed\n", dsCreate, dsStart, dsStop); return 1; }
        CGDSRef st = dsCreate(did, w, h, 'BGRA', NULL, q,
            ^(CGDSStatus status, uint64_t t, IOSurfaceRef frame, CGDSUpdateRef update) {
                if (status != kCGDSFrameComplete || !frame) return;
                IOSurfaceLock(frame, kIOSurfaceLockReadOnly, NULL);
                const uint8_t *src = (const uint8_t *)IOSurfaceGetBaseAddress(frame); size_t sbpr = IOSurfaceGetBytesPerRow(frame);
                unsigned fh = (unsigned)IOSurfaceGetHeight(frame), fw = (unsigned)IOSurfaceGetWidth(frame);
                for (unsigned y = 0; y < fh && y < h; y++) memcpy((void *)((uint8_t *)addr + (size_t)y * pitch), src + (size_t)y * sbpr, (fw < w ? fw : w) * 4);
                IOSurfaceUnlock(frame, kIOSurfaceLockReadOnly, NULL);
                frames++; bytes += (size_t)fh * fw * 4;
            });
        if (!st) { printf("CGDisplayStreamCreate failed (Screen Recording permission? TCC)\n"); return 1; }
        CGError e = dsStart(st);
        printf("-- display stream started (%d); mirroring for %u s — the macOS desktop is on the Wacom now\n", (int)e, seconds);
        int c0 = kms_crc(dev, hDev, hDisp, head, "t+1s"); sleep(1);
        for (unsigned i = 0; i < seconds; i++) { sleep(1); if (i % 5 == 4) { printf("   %lu frames, %lu MB copied\n", frames, bytes >> 20); kms_crc(dev, hDev, hDisp, head, "now"); } }
        int c1 = kms_crc(dev, hDev, hDisp, head, "end");
        dsStop(st);
        printf("-- %s: %lu frames from WindowServer reached the scanout (CRC %08x -> %08x)\n", frames ? "PASS" : "FAIL", frames, (unsigned)c0, (unsigned)c1);
        return frames ? 0 : 1;
    }
}
