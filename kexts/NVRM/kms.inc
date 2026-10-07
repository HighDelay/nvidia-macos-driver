/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#include "nvkms-api.h"
#define NV_VERSION_STRING "610.57.04"
#define SEL_KMS 4
static int kms_ioctl(io_connect_t c, uint32_t cmd, void *params, size_t size, const char *what)
{
    uint64_t in = cmd; size_t out = size;
    kern_return_t kr = IOConnectCallMethod(c, SEL_KMS, &in, 1, params, size, NULL, NULL, params, &out);
    if (kr != KERN_SUCCESS) { printf("  %-28s -> FAILED (kr 0x%08x)\n", what, kr); return -1; }
    return 0;
}
static int cmd_kms(void)
{
    io_connect_t ctl = nv_open(0), dev = nv_open(1);
    nv_ioctl_card_info_t ci; memset(&ci, 0, sizeof ci);
    if (nv_esc(ctl, NV_ESC_CARD_INFO, &ci, sizeof ci) != 0 || !ci.valid) { printf("no card info\n"); return 1; }
    printf("-- NVKMS on macOS: gpu_id 0x%x, driver %s\n", ci.gpu_id, NV_VERSION_STRING);
    struct NvKmsAllocDeviceParams ad; memset(&ad, 0, sizeof ad);
    strncpy(ad.request.versionString, NV_VERSION_STRING, sizeof ad.request.versionString - 1);
    ad.request.deviceId.rmDeviceId = 0;
    if (kms_ioctl(dev, NVKMS_IOCTL_ALLOC_DEVICE, &ad, sizeof ad, "ALLOC_DEVICE")) return 1;
    printf("  ALLOC_DEVICE status %d  deviceHandle %u  subDeviceMask 0x%x  numHeads %u  numDisps %u  swapGroups %u\n",
           ad.reply.status, ad.reply.deviceHandle, ad.reply.subDeviceMask, ad.reply.numHeads, ad.reply.numDisps, ad.reply.supportsSwapGroups);
    if (ad.reply.status != NVKMS_ALLOC_DEVICE_STATUS_SUCCESS) return 1;
    int connectedDpys = 0;
    for (NvU32 d = 0; d < ad.reply.numDisps; d++) {
        struct NvKmsQueryDispParams qd; memset(&qd, 0, sizeof qd);
        qd.request.deviceHandle = ad.reply.deviceHandle; qd.request.dispHandle = ad.reply.dispHandles[d];
        if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_DISP, &qd, sizeof qd, "QUERY_DISP")) return 1;
        printf("  disp %u (%s): connectors %u  validDpys 0x%x  bootDpys 0x%x\n", d, qd.reply.gpuString, qd.reply.numConnectors,
               nvDpyIdListToNvU32(qd.reply.validDpys), nvDpyIdListToNvU32(qd.reply.bootDpys));
        for (NvU32 c = 0; c < qd.reply.numConnectors; c++) {
            struct NvKmsQueryConnectorStaticDataParams qc; memset(&qc, 0, sizeof qc);
            qc.request.deviceHandle = ad.reply.deviceHandle; qc.request.dispHandle = ad.reply.dispHandles[d]; qc.request.connectorHandle = qd.reply.connectorHandles[c];
            if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_CONNECTOR_STATIC_DATA, &qc, sizeof qc, "QUERY_CONNECTOR_STATIC_DATA")) continue;
            printf("    connector %u: type %d typeIndex %u  isDP %u  dpyId 0x%x  physicalIndex %u\n", c, qc.reply.type, qc.reply.typeIndex, qc.reply.isDP, nvDpyIdToNvU32(qc.reply.dpyId), qc.reply.physicalIndex);
        }
        for (unsigned bit = 0; bit < 32; bit++) {
            if (!(nvDpyIdListToNvU32(qd.reply.validDpys) & (1u << bit))) continue;
            struct NvKmsQueryDpyStaticDataParams qs; memset(&qs, 0, sizeof qs);
            qs.request.deviceHandle = ad.reply.deviceHandle; qs.request.dispHandle = ad.reply.dispHandles[d]; qs.request.dpyId = nvNvU32ToDpyId(1u << bit);
            if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_STATIC_DATA, &qs, sizeof qs, "QUERY_DPY_STATIC_DATA")) continue;
            struct NvKmsQueryDpyDynamicDataParams *qy = calloc(1, sizeof *qy);
            qy->request.deviceHandle = ad.reply.deviceHandle; qy->request.dispHandle = ad.reply.dispHandles[d]; qy->request.dpyId = nvNvU32ToDpyId(1u << bit);
            if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_DYNAMIC_DATA, qy, sizeof *qy, "QUERY_DPY_DYNAMIC_DATA")) { free(qy); continue; }
            printf("    dpy 0x%x: connector %u type 0x%x headMask 0x%x  %s  name \"%s\"  max %ux%u  %ux%u cm  edid %u B",
                   1u << bit, qs.reply.connectorHandle, qs.reply.type, qs.reply.headMask, qy->reply.connected ? "CONNECTED" : "not connected",
                   qy->reply.name, qy->reply.maxWidthInPixels, qy->reply.maxHeightInPixels, qy->reply.physicalDimensions.widthInCM,
                   qy->reply.physicalDimensions.heightInCM, qy->reply.edid.bufferSize);
            if (qy->reply.edid.bufferSize) { printf("  [");
                for (unsigned i = 0; i < 16 && i < qy->reply.edid.bufferSize; i++) printf("%02x", qy->reply.edid.buffer[i]); printf("...]"); }
            printf("\n");
            if (qy->reply.connected) connectedDpys++;
            free(qy);
        }
    }
    printf("-- %d connected display(s)\n", connectedDpys);
    return connectedDpys ? 0 : 1;
}

static void paint(volatile uint32_t *px, unsigned w, unsigned h, unsigned pitchPx, int phase)
{
    for (unsigned y = 0; y < h; y++) for (unsigned x = 0; x < w; x++) {
        uint32_t c;
        if (y < h / 8) c = ((x * 255 / w) << 16) | ((x * 255 / w) << 8) | (x * 255 / w);
        else if (x < w / 3) c = phase ? 0x0000ff : 0xff0000;
        else if (x < 2 * w / 3) c = 0x00ff00;
        else c = phase ? 0xff0000 : 0x0000ff;
        if (x < 8 || y < 8 || x >= w - 8 || y >= h - 8) c = 0x000000;
        if ((x * h) / w == y || (x * h) / w + 1 == y) c = 0xffffff;
        px[y * pitchPx + x] = c;
    }
}
static int kms_crc(io_connect_t dev, NvKmsDeviceHandle hDev, NvKmsDispHandle hDisp, NvU32 head, const char *label)
{
    struct NvKmsQueryDpyCRC32Params p; memset(&p, 0, sizeof p);
    p.request.deviceHandle = hDev; p.request.dispHandle = hDisp; p.request.head = head;
    if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_CRC32, &p, sizeof p, "QUERY_DPY_CRC32")) return -1;
    printf("  CRC32 %-10s compositor %s0x%08x  rasterGenerator %s0x%08x\n", label, p.reply.compositorCrc32.supported ? "" : "(unsupported) ",
           p.reply.compositorCrc32.value, p.reply.rasterGeneratorCrc32.supported ? "" : "(unsupported) ", p.reply.rasterGeneratorCrc32.value);
    return (int)p.reply.compositorCrc32.value;
}
static int cmd_kms_set(unsigned holdSeconds)
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
    printf("  device caps: surfaceAlignment %u  maxWidthInBytes %u  max %ux%u  layer0 formats 0x%llx (X8R8G8B8 bit %u: %s)  requiresAllAllocationsInSysmem %u  headSurface %u\n",
           ad.reply.surfaceAlignment, ad.reply.maxWidthInBytes, ad.reply.maxWidthInPixels, ad.reply.maxHeightInPixels,
           (unsigned long long)ad.reply.layerCaps[0].supportedSurfaceMemoryFormats, NvKmsSurfaceMemoryFormatX8R8G8B8,
           (ad.reply.layerCaps[0].supportedSurfaceMemoryFormats >> NvKmsSurfaceMemoryFormatX8R8G8B8) & 1 ? "yes" : "NO",
           ad.reply.requiresAllAllocationsInSysmem, ad.reply.supportsHeadSurface);
    struct NvKmsQueryDispParams qd; memset(&qd, 0, sizeof qd); qd.request.deviceHandle = hDev; qd.request.dispHandle = hDisp;
    if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_DISP, &qd, sizeof qd, "QUERY_DISP")) return 1;
    NVDpyId dpy = nvInvalidDpyId(); NvU32 headMask = 0;
    for (unsigned bit = 0; bit < 32 && nvDpyIdIsInvalid(dpy); bit++) {
        if (!(nvDpyIdListToNvU32(qd.reply.validDpys) & (1u << bit))) continue;
        struct NvKmsQueryDpyDynamicDataParams *qy = calloc(1, sizeof *qy);
        qy->request.deviceHandle = hDev; qy->request.dispHandle = hDisp; qy->request.dpyId = nvNvU32ToDpyId(1u << bit);
        if (!kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_DYNAMIC_DATA, qy, sizeof *qy, "QUERY_DPY_DYNAMIC_DATA") && qy->reply.connected) {
            dpy = qy->request.dpyId; printf("-- using dpy 0x%x \"%s\"\n", 1u << bit, qy->reply.name);
            struct NvKmsQueryDpyStaticDataParams qs; memset(&qs, 0, sizeof qs);
            qs.request.deviceHandle = hDev; qs.request.dispHandle = hDisp; qs.request.dpyId = dpy;
            if (!kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_STATIC_DATA, &qs, sizeof qs, "QUERY_DPY_STATIC_DATA")) headMask = qs.reply.headMask;
        }
        free(qy);
    }
    if (nvDpyIdIsInvalid(dpy)) { printf("no connected display\n"); return 1; }
    NvU32 head = 0; while (head < 32 && !(headMask & (1u << head))) head++;

    struct NvKmsValidateModeIndexParams *vm = calloc(1, sizeof *vm); struct NvKmsMode mode; int haveMode = 0, nModes = 0;
    for (NvU32 i = 0; i < 256; i++) {
        memset(vm, 0, sizeof *vm); vm->request.deviceHandle = hDev; vm->request.dispHandle = hDisp; vm->request.dpyId = dpy; vm->request.modeIndex = i;
        if (kms_ioctl(dev, NVKMS_IOCTL_VALIDATE_MODE_INDEX, vm, sizeof *vm, "VALIDATE_MODE_INDEX")) break;
        if (vm->reply.end) break;
        if (vm->reply.valid) {
            nModes++;
            if (nModes <= 10) printf("  mode %2u: %ux%u @%u.%03u Hz  clock %u kHz  %s%s\n", i, vm->reply.mode.timings.hVisible, vm->reply.mode.timings.vVisible,
                   vm->reply.mode.timings.RRx1k / 1000, vm->reply.mode.timings.RRx1k % 1000, vm->reply.mode.timings.pixelClockHz / 1000, vm->reply.mode.name, vm->reply.preferredMode ? "  PREFERRED" : "");
            if (!haveMode || vm->reply.preferredMode) { mode = vm->reply.mode; haveMode = 1; }
            if (vm->reply.preferredMode) { break; }
        }
    }
    free(vm);
    if (!haveMode) { printf("no valid mode\n"); return 1; }
    unsigned w = mode.timings.hVisible, h = mode.timings.vVisible; unsigned pitch = (w * 4 + 255) & ~255u; NvU64 size = ((NvU64)pitch * h + 0xffff) & ~0xffffull;
    printf("-- surface %ux%u pitch %u (%llu KB VRAM), head %u\n", w, h, pitch, (unsigned long long)size / 1024, head);

    NV_MEMORY_ALLOCATION_PARAMS mp; memset(&mp, 0, sizeof mp);
    mp.owner = hClient; mp.type = NVOS32_TYPE_IMAGE; mp.size = size; mp.alignment = 65536;
    mp.attr = DRF_DEF(OS32, _ATTR, _PAGE_SIZE, _4KB) | DRF_DEF(OS32, _ATTR, _LOCATION, _VIDMEM) | DRF_DEF(OS32, _ATTR, _COHERENCY, _UNCACHED) | DRF_DEF(OS32, _ATTR, _FORMAT, _PITCH);
    mp.flags = NVOS32_ALLOC_FLAGS_ALIGNMENT_FORCE;
    NvHandle hMem = rm_alloc(ctl, hClient, hDevice, 0xcafe0031, NV01_MEMORY_LOCAL_USER, &mp, sizeof mp, "NV01_MEMORY_LOCAL_USER (surface)"); if (!hMem) return 1;
    io_connect_t mc = nv_open(1);
    nv_ioctl_nvos33_parameters_with_fd m; memset(&m, 0, sizeof m);
    m.params.hClient = hClient; m.params.hDevice = hSub; m.params.hMemory = hMem; m.params.length = size; m.fd = nv_fd(mc);
    if (nv_esc(ctl, NV_ESC_RM_MAP_MEMORY, &m, sizeof m) != 0 || m.params.status != 0) { printf("map surface failed 0x%x\n", m.params.status); return 1; }
    mach_vm_address_t addr = 0; mach_vm_size_t len = 0;
    if (IOConnectMapMemory64(mc, 0, mach_task_self(), &addr, &len, kIOMapAnywhere) != KERN_SUCCESS) { printf("IOConnectMapMemory64 failed\n"); return 1; }
    volatile uint32_t *px = (volatile uint32_t *)(uintptr_t)addr;
    paint(px, w, h, pitch / 4, 0);
    printf("  painted phase 0 through BAR1 at 0x%llx (%llu KB)\n", (unsigned long long)addr, (unsigned long long)len / 1024);

    struct NvKmsRegisterSurfaceParams rs; memset(&rs, 0, sizeof rs);
    rs.request.deviceHandle = hDev; rs.request.useFd = NV_FALSE; rs.request.rmClient = hClient;
    rs.request.planes[0].u.rmObject = hMem; rs.request.planes[0].offset = 0; rs.request.planes[0].pitch = pitch; rs.request.planes[0].rmObjectSizeInBytes = size;
    rs.request.widthInPixels = w; rs.request.heightInPixels = h; rs.request.layout = NvKmsSurfaceMemoryLayoutPitch; rs.request.format = NvKmsSurfaceMemoryFormatX8R8G8B8;
    rs.request.isoType = NVKMS_MEMORY_ISO;
    if (kms_ioctl(dev, NVKMS_IOCTL_REGISTER_SURFACE, &rs, sizeof rs, "REGISTER_SURFACE")) return 1;
    printf("  REGISTER_SURFACE -> surfaceHandle %u\n", rs.reply.surfaceHandle);
    if (!rs.reply.surfaceHandle) return 1;

    struct NvKmsGrabOwnershipParams go; memset(&go, 0, sizeof go); go.request.deviceHandle = hDev;
    if (kms_ioctl(dev, NVKMS_IOCTL_GRAB_OWNERSHIP, &go, sizeof go, "GRAB_OWNERSHIP")) return 1;
    printf("  GRAB_OWNERSHIP ok\n");

    struct NvKmsSetModeParams *sm = calloc(1, sizeof *sm);
    sm->request.deviceHandle = hDev; sm->request.commit = NV_TRUE; sm->request.requestedDispsBitMask = 1;
    sm->request.disp[0].requestedHeadsBitMask = 1u << head;
    struct NvKmsSetModeOneHeadRequest *hr = &sm->request.disp[0].head[head];
    hr->dpyIdList = nvAddDpyIdToEmptyDpyIdList(dpy); hr->mode = mode;
    hr->viewPortSizeIn.width = w; hr->viewPortSizeIn.height = h;
    hr->viewPortOut.x = 0; hr->viewPortOut.y = 0; hr->viewPortOut.width = w; hr->viewPortOut.height = h; hr->viewPortOutSpecified = NV_TRUE;
    hr->flip.layer[NVKMS_MAIN_LAYER].surface.specified = NV_TRUE; hr->flip.layer[NVKMS_MAIN_LAYER].surface.handle[NVKMS_LEFT] = rs.reply.surfaceHandle;
    hr->flip.layer[NVKMS_MAIN_LAYER].sizeIn.specified = NV_TRUE;  hr->flip.layer[NVKMS_MAIN_LAYER].sizeIn.val.width = w;  hr->flip.layer[NVKMS_MAIN_LAYER].sizeIn.val.height = h;
    hr->flip.layer[NVKMS_MAIN_LAYER].sizeOut.specified = NV_TRUE; hr->flip.layer[NVKMS_MAIN_LAYER].sizeOut.val.width = w; hr->flip.layer[NVKMS_MAIN_LAYER].sizeOut.val.height = h;
    int rc = kms_ioctl(dev, NVKMS_IOCTL_SET_MODE, sm, sizeof *sm, "SET_MODE");
    printf("  SET_MODE -> ioctl %s, status %d, disp status %d, head status %d\n", rc ? "FAILED" : "ok", sm->reply.status, sm->reply.disp[0].status, sm->reply.disp[0].head[head].status);
    if (rc || sm->reply.status != NVKMS_SET_MODE_STATUS_SUCCESS) return 1;

    sleep(1); int c0 = kms_crc(dev, hDev, hDisp, head, "phase 0");
    paint(px, w, h, pitch / 4, 1); sleep(1); int c1 = kms_crc(dev, hDev, hDisp, head, "phase 1");
    paint(px, w, h, pitch / 4, 0); sleep(1); int c2 = kms_crc(dev, hDev, hDisp, head, "phase 0 again");
    printf("-- %s: the display engine scans out our surface (CRC phase0 %08x != phase1 %08x, phase0 again %08x)\n",
           (c0 != c1 && c0 == c2 && c0 != -1) ? "PASS" : "UNPROVEN", (unsigned)c0, (unsigned)c1, (unsigned)c2);
    printf("-- holding the mode for %u s (look at the monitor: red | green | blue bars, grey ramp on top, white diagonal, black frame)\n", holdSeconds);
    sleep(holdSeconds);
    struct NvKmsReleaseOwnershipParams ro; memset(&ro, 0, sizeof ro); ro.request.deviceHandle = hDev;
    kms_ioctl(dev, NVKMS_IOCTL_RELEASE_OWNERSHIP, &ro, sizeof ro, "RELEASE_OWNERSHIP");
    return (c0 != c1 && c0 == c2) ? 0 : 1;
}

static int cmd_fbopen(unsigned seconds)
{
    io_service_t fb = IOServiceGetMatchingService(kIOMainPortDefault, IOServiceMatching("NVRMFramebuffer"));
    if (!fb) { printf("no NVRMFramebuffer service\n"); return 1; }
    io_connect_t c = 0; kern_return_t kr = IOServiceOpen(fb, mach_task_self(), 0, &c);
    printf("IOServiceOpen(NVRMFramebuffer, type 0) -> 0x%08x conn %u\n", kr, c);
    if (kr != KERN_SUCCESS) { kr = IOServiceOpen(fb, mach_task_self(), 1, &c); printf("IOServiceOpen(type 1) -> 0x%08x\n", kr); }
    if (kr == KERN_SUCCESS) { printf("holding %u s\n", seconds); sleep(seconds); IOServiceClose(c); }
    return kr == KERN_SUCCESS ? 0 : 1;
}

static int cmd_kms_crc(unsigned seconds)
{
    io_connect_t dev = nv_open(1);
    struct NvKmsAllocDeviceParams ad; memset(&ad, 0, sizeof ad);
    strncpy(ad.request.versionString, NV_VERSION_STRING, sizeof ad.request.versionString - 1); ad.request.deviceId.rmDeviceId = 0;
    if (kms_ioctl(dev, NVKMS_IOCTL_ALLOC_DEVICE, &ad, sizeof ad, "ALLOC_DEVICE") || ad.reply.status != NVKMS_ALLOC_DEVICE_STATUS_SUCCESS) { printf("alloc device status %d\n", ad.reply.status); return 1; }
    unsigned distinct = 0, changes = 0; uint32_t last = 0; uint32_t seen[64]; unsigned nseen = 0;
    for (unsigned i = 0; i < seconds; i++) {
        struct NvKmsQueryDpyCRC32Params p; memset(&p, 0, sizeof p);
        p.request.deviceHandle = ad.reply.deviceHandle; p.request.dispHandle = ad.reply.dispHandles[0]; p.request.head = 0;
        if (kms_ioctl(dev, NVKMS_IOCTL_QUERY_DPY_CRC32, &p, sizeof p, "QUERY_DPY_CRC32")) return 1;
        uint32_t c = p.reply.compositorCrc32.value;
        if (i && c != last) changes++;
        unsigned k; for (k = 0; k < nseen && seen[k] != c; k++); if (k == nseen && nseen < 64) { seen[nseen++] = c; distinct++; }
        if (i < 5 || c != last) printf("  t=%3us  compositor CRC 0x%08x%s  (supported %u, raster 0x%08x)\n", i, c, (i && c != last) ? "  CHANGED" : "", p.reply.compositorCrc32.supported, p.reply.rasterGeneratorCrc32.value);
        last = c; sleep(1);
    }
    printf("-- %u samples, %u distinct CRCs, %u changes: %s\n", seconds, distinct, changes, changes ? "the scanout is being repainted by someone else" : "static frame");
    return 0;
}

static int cmd_fbpeek(unsigned seconds)
{
    io_service_t fb = IOServiceGetMatchingService(kIOMainPortDefault, IOServiceMatching("NVRMFramebuffer"));
    if (!fb) { printf("no NVRMFramebuffer service\n"); return 1; }
    io_connect_t c = 0; kern_return_t kr = IOServiceOpen(fb, mach_task_self(), 1 , &c);
    printf("IOServiceOpen(shared) -> 0x%08x\n", kr); if (kr) return 1;
    mach_vm_address_t addr = 0; mach_vm_size_t len = 0;
    kr = IOConnectMapMemory64(c, 110 , mach_task_self(), &addr, &len, kIOMapAnywhere);
    printf("map VRAM aperture -> 0x%08x addr 0x%llx len %llu KB\n", kr, (unsigned long long)addr, (unsigned long long)len / 1024); if (kr) return 1;
    const unsigned w = 1920, h = 1080, pitch = 7680; volatile uint32_t *px = (volatile uint32_t *)(uintptr_t)addr;
    uint32_t lastSum = 0;
    for (unsigned t = 0; t < seconds; t += 5) {
        uint32_t sum = 0; unsigned notGrey = 0, dark = 0; uint32_t hist[4] = {0};
        for (unsigned y = 0; y < h; y += 8) for (unsigned x = 0; x < w; x += 8) { uint32_t v = px[y * (pitch / 4) + x] & 0xffffff; sum = sum * 33 + v; if (v != 0x303030) notGrey++; if (v < 0x101010) dark++; hist[(v >> 22) & 3]++; }
        printf("  t=%3us  sample-sum 0x%08x%s  not-grey %u/%u  near-black %u  px(100,100)=0x%06x px(960,540)=0x%06x px(1800,1000)=0x%06x\n", t, sum, (t && sum != lastSum) ? "  CHANGED" : "", notGrey, (h / 8) * (w / 8), dark,
               px[100 * (pitch / 4) + 100] & 0xffffff, px[540 * (pitch / 4) + 960] & 0xffffff, px[1000 * (pitch / 4) + 1800] & 0xffffff);
        lastSum = sum; sleep(5);
    }
    IOConnectUnmapMemory64(c, 110, mach_task_self(), addr); IOServiceClose(c);
    return 0;
}
