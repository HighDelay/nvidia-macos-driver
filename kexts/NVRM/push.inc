/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

struct gbuf { NvHandle hPhys, hVirt; NvU64 gpuva; volatile void *cpu; NvU64 size; io_connect_t mc; };
static NvHandle gNext = 0xcafe0100;

static int gbuf_alloc(io_connect_t ctl, NvHandle hClient, NvHandle hDevice, NvHandle hSub, NvHandle hVaSpace,
                      int vidmem, NvU64 size, const char *what, struct gbuf *b)
{
    memset(b, 0, sizeof *b); b->size = size;
    NV_MEMORY_ALLOCATION_PARAMS mp; memset(&mp, 0, sizeof mp);
    mp.owner = hClient; mp.type = NVOS32_TYPE_IMAGE; mp.size = size; mp.attr = DRF_DEF(OS32, _ATTR, _PAGE_SIZE, _4KB);
    if (vidmem) { mp.attr |= DRF_DEF(OS32, _ATTR, _LOCATION, _VIDMEM) | DRF_DEF(OS32, _ATTR, _COHERENCY, _UNCACHED); mp.flags |= NVOS32_ALLOC_FLAGS_PERSISTENT_VIDMEM; }
    else        { mp.attr |= DRF_DEF(OS32, _ATTR, _LOCATION, _PCI)    | DRF_DEF(OS32, _ATTR, _COHERENCY, _CACHED); }
    b->hPhys = rm_alloc(ctl, hClient, hDevice, gNext++, vidmem ? NV01_MEMORY_LOCAL_USER : NV01_MEMORY_SYSTEM, &mp, sizeof mp, what);
    if (!b->hPhys) return 1;
    b->mc = nv_open(vidmem ? 1 : 0);
    nv_ioctl_nvos33_parameters_with_fd m; memset(&m, 0, sizeof m);
    m.params.hClient = hClient; m.params.hDevice = hSub; m.params.hMemory = b->hPhys; m.params.length = size; m.fd = nv_fd(b->mc);
    if (nv_esc(ctl, NV_ESC_RM_MAP_MEMORY, &m, sizeof m) != 0 || m.params.status != 0) { printf("  %s: map_memory status 0x%x\n", what, m.params.status); return 1; }
    mach_vm_address_t addr = 0; mach_vm_size_t len = 0;
    kern_return_t kr = IOConnectMapMemory64(b->mc, 0, mach_task_self(), &addr, &len, kIOMapAnywhere);
    if (kr != KERN_SUCCESS) { printf("  %s: IOConnectMapMemory64 0x%08x\n", what, kr); return 1; }
    b->cpu = (volatile void *)(uintptr_t)addr;
    NV_MEMORY_ALLOCATION_PARAMS vp; memset(&vp, 0, sizeof vp);
    vp.owner = hClient; vp.type = NVOS32_TYPE_IMAGE; vp.flags = NVOS32_ALLOC_FLAGS_VIRTUAL; vp.size = size; vp.hVASpace = hVaSpace;
    NVOS21_PARAMETERS a; memset(&a, 0, sizeof a);
    a.hRoot = hClient; a.hObjectParent = hDevice; a.hObjectNew = gNext++; a.hClass = CLS_NV50_MEMORY_VIRTUAL; a.pAllocParms = (NvP64)(uintptr_t)&vp; a.paramsSize = sizeof vp;
    if (nv_esc(ctl, NV_ESC_RM_ALLOC, &a, sizeof a) != 0 || a.status != 0) { printf("  %s: NV50_MEMORY_VIRTUAL status 0x%x\n", what, a.status); return 1; }
    b->hVirt = a.hObjectNew; b->gpuva = vp.offset;
    NVOS46_PARAMETERS d; memset(&d, 0, sizeof d);
    d.hClient = hClient; d.hDevice = hDevice; d.hDma = b->hVirt; d.hMemory = b->hPhys; d.offset = 0; d.length = size;
    d.flags = DRF_DEF(OS46, _FLAGS, _PAGE_KIND, _VIRTUAL) | (vidmem ? DRF_DEF(OS46, _FLAGS, _CACHE_SNOOP, _DISABLE) : DRF_DEF(OS46, _FLAGS, _CACHE_SNOOP, _ENABLE));
    d.dmaOffset = 0;
    if (nv_esc(ctl, NV_ESC_RM_MAP_MEMORY_DMA, &d, sizeof d) != 0 || d.status != 0) { printf("  %s: MAP_MEMORY_DMA status 0x%x\n", what, d.status); return 1; }
    printf("  buf %-9s %s %6llu KB  cpu %p  gpuva 0x%llx (dmaOffset 0x%llx)\n", what, vidmem ? "vidmem" : "sysmem", (unsigned long long)(size >> 10), (void *)b->cpu, (unsigned long long)b->gpuva, (unsigned long long)d.dmaOffset);
    return 0;
}

static int push_test(io_connect_t ctl, io_connect_t dev, NvHandle hClient, NvHandle hDevice, NvHandle hSub)
{
    NV0080_CTRL_GPU_GET_CLASSLIST_V2_PARAMS cl; memset(&cl, 0, sizeof cl);
    NvU32 st = rm_control(ctl, hClient, hDevice, NV0080_CTRL_CMD_GPU_GET_CLASSLIST_V2, &cl, sizeof cl);
    NvU32 chanClass = 0; printf("  classlist -> status 0x%x, %u classes:", st, cl.numClasses);
    for (NvU32 i = 0; i < cl.numClasses && i < NV0080_CTRL_GPU_CLASSLIST_MAX_SIZE; i++) { printf(" %x", cl.classList[i]); if ((cl.classList[i] & 0xff) == 0x6f && cl.classList[i] > chanClass) chanClass = cl.classList[i]; }
    printf("\n  channel class: 0x%x\n", chanClass);
    if (!chanClass) return 1;

    NvHandle hUsermode = rm_alloc(ctl, hClient, hSub, gNext++, CLS_TURING_USERMODE_A, NULL, 0, "TURING_USERMODE_A");
    if (!hUsermode) return 1;
    io_connect_t umc = nv_open(1);
    nv_ioctl_nvos33_parameters_with_fd um; memset(&um, 0, sizeof um);
    um.params.hClient = hClient; um.params.hDevice = hSub; um.params.hMemory = hUsermode; um.params.length = 4096; um.fd = nv_fd(umc);
    if (nv_esc(ctl, NV_ESC_RM_MAP_MEMORY, &um, sizeof um) != 0 || um.params.status != 0) { printf("  usermode map status 0x%x\n", um.params.status); return 1; }
    mach_vm_address_t uaddr = 0; mach_vm_size_t ulen = 0;
    if (IOConnectMapMemory64(umc, 0, mach_task_self(), &uaddr, &ulen, kIOMapAnywhere) != KERN_SUCCESS) { printf("  usermode IOConnectMapMemory64 failed\n"); return 1; }
    printf("  usermode doorbell page mapped at %p (pLinearAddress 0x%llx)\n", (void *)(uintptr_t)uaddr, (unsigned long long)um.params.pLinearAddress);

    NV_VASPACE_ALLOCATION_PARAMETERS vap; memset(&vap, 0, sizeof vap); vap.flags = NV_VASPACE_ALLOCATION_FLAGS_RETRY_PTE_ALLOC_IN_SYS;
    NvHandle hVaSpace = rm_alloc(ctl, hClient, hDevice, gNext++, CLS_FERMI_VASPACE_A, &vap, sizeof vap, "FERMI_VASPACE_A");
    if (!hVaSpace) return 1;

    struct gbuf notifier, userd, gpfifo, cmdbuf, sem;
    if (gbuf_alloc(ctl, hClient, hDevice, hSub, hVaSpace, 0, 0x1000,  "notifier", &notifier)) return 1;
    if (gbuf_alloc(ctl, hClient, hDevice, hSub, hVaSpace, 1, 0x80000, "userd",    &userd))    return 1;
    if (gbuf_alloc(ctl, hClient, hDevice, hSub, hVaSpace, 0, 0x40000, "gpfifo",   &gpfifo))   return 1;
    if (gbuf_alloc(ctl, hClient, hDevice, hSub, hVaSpace, 0, 0x10000, "cmdbuf",   &cmdbuf))   return 1;
    if (gbuf_alloc(ctl, hClient, hDevice, hSub, hVaSpace, 0, 0x1000,  "sem",      &sem))      return 1;
    memset((void *)sem.cpu, 0, 0x1000); memset((void *)notifier.cpu, 0, 0x1000);

    NV_CONTEXT_DMA_ALLOCATION_PARAMS cd; memset(&cd, 0, sizeof cd);
    cd.flags = DRF_DEF(OS03, _FLAGS, _MAPPING, _KERNEL) | DRF_DEF(OS03, _FLAGS, _HASH_TABLE, _DISABLE);
    cd.hMemory = notifier.hPhys; cd.offset = 0; cd.limit = notifier.size - 1;
    NvHandle hCtxDma = rm_alloc(ctl, hClient, hDevice, gNext++, CLS_NV01_CONTEXT_DMA, &cd, sizeof cd, "NV01_CONTEXT_DMA");
    if (!hCtxDma) return 1;

    NV_CHANNEL_ALLOC_PARAMS cp; memset(&cp, 0, sizeof cp);
    cp.hObjectError = hCtxDma; cp.gpFifoOffset = gpfifo.gpuva; cp.gpFifoEntries = 0x8000; cp.flags = 0; cp.hVASpace = hVaSpace;
    cp.hUserdMemory[0] = userd.hPhys; cp.userdOffset[0] = 0; cp.engineType = NV2080_ENGINE_TYPE_GRAPHICS;
    NvHandle hChan = rm_alloc(ctl, hClient, hDevice, gNext++, chanClass, &cp, sizeof cp, "GPFIFO channel");
    if (!hChan) return 1;

    NVA06F_CTRL_BIND_PARAMS bp; memset(&bp, 0, sizeof bp); bp.engineType = NV2080_ENGINE_TYPE_GRAPHICS;
    st = rm_control(ctl, hClient, hChan, NVA06F_CTRL_CMD_BIND, &bp, sizeof bp); printf("  bind(GRAPHICS) -> 0x%x\n", st); if (st) return 1;
    NVA06F_CTRL_GPFIFO_SCHEDULE_PARAMS sp; memset(&sp, 0, sizeof sp); sp.bEnable = NV_TRUE;
    st = rm_control(ctl, hClient, hChan, NVA06F_CTRL_CMD_GPFIFO_SCHEDULE, &sp, sizeof sp); printf("  schedule -> 0x%x\n", st); if (st) return 1;
    NVC36F_CTRL_GPFIFO_SET_WORK_SUBMIT_TOKEN_NOTIF_INDEX_PARAMS ni; memset(&ni, 0, sizeof ni); ni.index = NV_CHANNELGPFIFO_NOTIFICATION_TYPE_WORK_SUBMIT_TOKEN;
    st = rm_control(ctl, hClient, hChan, NVC36F_CTRL_CMD_GPFIFO_SET_WORK_SUBMIT_TOKEN_NOTIF_INDEX, &ni, sizeof ni); printf("  set token notif index -> 0x%x\n", st);
    NVC36F_CTRL_CMD_GPFIFO_GET_WORK_SUBMIT_TOKEN_PARAMS tk; memset(&tk, 0, sizeof tk);
    st = rm_control(ctl, hClient, hChan, NVC36F_CTRL_CMD_GPFIFO_GET_WORK_SUBMIT_TOKEN, &tk, sizeof tk); printf("  work submit token -> 0x%x token 0x%08x\n", st, tk.workSubmitToken); if (st) return 1;

    volatile uint32_t *pb = (volatile uint32_t *)cmdbuf.cpu;
    pb[0] = 0x20000000u | (5u << 16) | (0u << 13) | (0x5cu >> 2);
    pb[1] = (uint32_t)sem.gpuva & ~3u;
    pb[2] = (uint32_t)(sem.gpuva >> 32) & 0xffu;
    pb[3] = 1u; pb[4] = 0u;
    pb[5] = 1u  | (1u << 20)  | (1u << 24) ;
    uint32_t ndw = 6;
    volatile uint32_t *gp = (volatile uint32_t *)gpfifo.cpu;
    gp[0] = (uint32_t)cmdbuf.gpuva & ~3u;
    gp[1] = ((uint32_t)(cmdbuf.gpuva >> 32) & 0xffu) | (ndw << 10);
    __sync_synchronize();
    volatile uint32_t *ud = (volatile uint32_t *)userd.cpu;
    ud[USERD_GPPUT / 4] = 1;
    __sync_synchronize();
    *(volatile uint32_t *)((uint8_t *)(uintptr_t)uaddr + USERMODE_NOTIFY_CHANNEL_PENDING) = tk.workSubmitToken;
    printf("  doorbell rung. waiting for the semaphore…\n");
    volatile uint64_t *semv = (volatile uint64_t *)sem.cpu; int ok = 0;
    for (int i = 0; i < 5000; i++) { if (*semv == 1) { ok = 1; break; } usleep(1000); }
    printf("  semaphore = %llu after %s; USERD GPGet=%u GPPut=%u\n", (unsigned long long)*semv, ok ? "<5 s" : "5 s timeout", ud[USERD_GPGET / 4], ud[USERD_GPPUT / 4]);
    printf(ok ? "PASS 5 REACHED: the GPU executed a semaphore release under macOS \n" : "PASS 5 NOT REACHED \n");
    return ok ? 0 : 1;
}
