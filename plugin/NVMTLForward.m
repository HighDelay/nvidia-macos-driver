/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#include <unistd.h>

#import <objc/runtime.h>
#import <objc/message.h>
#import <os/lock.h>

static os_unfair_lock  gFwdLock = OS_UNFAIR_LOCK_INIT;
static CFMutableDictionaryRef gEncCache;
static CFMutableDictionaryRef gSeen;
static int gIndexBuilt = 0;

typedef NSMethodSignature *(*nvfwd_msfs_fn)(id, SEL, SEL);
static struct { __unsafe_unretained Class cls; nvfwd_msfs_fn orig; } gOrig[32];
static int gNOrig = 0;

static nvfwd_msfs_fn nvfwd_original_for(Class c)
{
    for (Class k = c; k; k = class_getSuperclass(k))
        for (int i = 0; i < gNOrig; i++)
            if (gOrig[i].cls == k) return gOrig[i].orig;
    return NULL;
}

static const char *const kNVCounterpart[][2] = {
    {"NVMTLDevice",                 "MTLIOAccelDevice"},
    {"NVMTLCommandQueue",           "MTLIOAccelCommandQueue"},
    {"NVMTLCommandBuffer",          "MTLIOAccelCommandBuffer"},
    {"NVMTLBuffer",                 "MTLIOAccelBuffer"},
    {"NVMTLTexture",                "MTLIOAccelTexture"},
    {"NVMTLRenderCommandEncoder",   "MTLIOAccelRenderCommandEncoder"},
    {"NVMTLBlitCommandEncoder",     "MTLIOAccelBlitCommandEncoder"},
    {"NVMTLAccelerationStructureCommandEncoder", "_MTLAccelerationStructureCommandEncoder"},
    {"NVMTLAccelerationStructure",  "MTLIOAccelAccelerationStructure"},
    {"NVMTLComputeCommandEncoder",  "MTLIOAccelComputeCommandEncoder"},
    {"NVMTLHeap",                   "MTLIOAccelHeap"},
    {"NVMTLFence",                  "MTLIOAccelFence"},
    {"NVMTLSamplerState",           "MTLIOAccelSamplerState"},
    {"NVMTLDepthStencilState",      "MTLIOAccelDepthStencilState"},
    {"NVMTLLibrary",                "MTLLibrary"},
    {"NVMTLFunction",               "MTLFunction"},
    {"NVMTLArgumentEncoder",        "MTLArgumentEncoder"},
    {"NVMTLRenderPipelineState",    "MTLRenderPipelineState"},
    {"NVMTLComputePipelineState",   "MTLComputePipelineState"},
    {"NVMTLIndirectCommandBuffer",  "MTLIndirectCommandBuffer"},
    {NULL, NULL}
};

static const char *nvfwd_counterpart_encoding(Class c, SEL sel)
{
    const char *cn = class_getName(c);
    if (!cn) return NULL;
    for (int i = 0; kNVCounterpart[i][0]; i++) {
        if (strcmp(cn, kNVCounterpart[i][0])) continue;
        const char *an = kNVCounterpart[i][1];
        Class ac = objc_getClass(an);
        if (ac) {
            Method m = class_getInstanceMethod(ac, sel);
            if (m) return method_getTypeEncoding(m);
        }
        Protocol *p = objc_getProtocol(an);
        if (p) {
            for (int req = 0; req < 2; req++)
                for (int inst = 0; inst < 1; inst++) {
                    struct objc_method_description d =
                        protocol_getMethodDescription(p, sel, req == 0, YES);
                    if (d.name && d.types) return d.types;
                }
        }
        return NULL;
    }
    return NULL;
}

static void nvfwd_index_class(Class c)
{
    unsigned mc = 0;
    Method *ms = class_copyMethodList(c, &mc);
    for (unsigned j = 0; j < mc; j++) {
        const char *sn = sel_getName(method_getName(ms[j]));
        const char *te = method_getTypeEncoding(ms[j]);
        if (!sn || !te) continue;
        CFStringRef k = CFStringCreateWithCString(NULL, sn, kCFStringEncodingUTF8);
        if (k) {
            if (!CFDictionaryContainsKey(gEncCache, k)) {
                CFStringRef v = CFStringCreateWithCString(NULL, te, kCFStringEncodingUTF8);
                if (v) { CFDictionarySetValue(gEncCache, k, v); CFRelease(v); }
            }
            CFRelease(k);
        }
    }
    free(ms);
}

static void nvfwd_build_index(void)
{
    if (gIndexBuilt) return;
    gIndexBuilt = 1;
    int nclass = 0, nproto = 0;

    unsigned n = 0;
    Class *all = objc_copyClassList(&n);
    for (unsigned i = 0; i < n; i++) {
        const char *cn = class_getName(all[i]);
        if (!cn) continue;
        if (strncmp(cn, "MTL", 3) && strncmp(cn, "AGX", 3) && strncmp(cn, "AMD", 3) &&
            strncmp(cn, "IOAccel", 7) && strncmp(cn, "CA", 2) && strncmp(cn, "Apple", 5)) continue;
        nvfwd_index_class(all[i]);
        nclass++;
    }
    free(all);

    unsigned np = 0;
    Protocol *__unsafe_unretained *ps = objc_copyProtocolList(&np);
    for (unsigned i = 0; i < np; i++) {
        const char *pn = protocol_getName(ps[i]);
        if (!pn || strncmp(pn, "MTL", 3)) continue;
        nproto++;
        for (int req = 0; req < 2; req++) {
            unsigned dc = 0;
            struct objc_method_description *ds =
                protocol_copyMethodDescriptionList(ps[i], req == 0, YES, &dc);
            for (unsigned j = 0; j < dc; j++) {
                const char *sn = ds[j].name ? sel_getName(ds[j].name) : NULL;
                const char *te = ds[j].types;
                if (!sn || !te) continue;
                CFStringRef k = CFStringCreateWithCString(NULL, sn, kCFStringEncodingUTF8);
                if (k) {
                    if (!CFDictionaryContainsKey(gEncCache, k)) {
                        CFStringRef v = CFStringCreateWithCString(NULL, te, kCFStringEncodingUTF8);
                        if (v) { CFDictionarySetValue(gEncCache, k, v); CFRelease(v); }
                    }
                    CFRelease(k);
                }
            }
            free(ds);
        }
    }
    free(ps);

    nvlog("NVMTLForward: encoding index built from %d classes + %d protocols, %ld selectors",
          nclass, nproto, (long)CFDictionaryGetCount(gEncCache));
}

static NSMethodSignature *nvfwd_synthesize(SEL sel)
{
    const char *sn = sel_getName(sel);
    int colons = 0;
    for (const char *p = sn; *p; p++) if (*p == ':') colons++;
    char enc[80];
    if (colons > 60) return nil;
    int o = 0;
    enc[o++] = '@'; enc[o++] = '@'; enc[o++] = ':';
    for (int i = 0; i < colons; i++) enc[o++] = '@';
    enc[o] = 0;
    nvlog("NVMTLForward: SYNTHESIZED signature \"%s\" for %s — no class or protocol declares it", enc, sn);
    return [NSMethodSignature signatureWithObjCTypes:enc];
}

static NSMethodSignature *nvfwd_msfs(id self, SEL _cmd, SEL sel)
{
    nvfwd_msfs_fn orig = nvfwd_original_for(object_getClass(self));
    if (orig) {
        NSMethodSignature *s = orig(self, _cmd, sel);
        if (s) return s;
    }

    const char *cte = nvfwd_counterpart_encoding(object_getClass(self), sel);
    if (cte) return [NSMethodSignature signatureWithObjCTypes:cte];

    const char *sn = sel_getName(sel);
    CFStringRef key = CFStringCreateWithCString(NULL, sn, kCFStringEncodingUTF8);
    if (!key) return nil;

    os_unfair_lock_lock(&gFwdLock);
    nvfwd_build_index();
    CFStringRef enc = (CFStringRef)CFDictionaryGetValue(gEncCache, key);
    os_unfair_lock_unlock(&gFwdLock);
    CFRelease(key);

    if (enc) {
        char buf[512];
        if (CFStringGetCString(enc, buf, sizeof buf, kCFStringEncodingUTF8))
            return [NSMethodSignature signatureWithObjCTypes:buf];
    }
    return nvfwd_synthesize(sel);
}

static void nvfwd_forward(id self, SEL _cmd, NSInvocation *inv)
{
    SEL sel = [inv selector];
    const char *cn = class_getName(object_getClass(self));
    const char *sn = sel_getName(sel);

    char key[320];
    snprintf(key, sizeof key, "%s %s", cn, sn);
    long count = 0;
    CFStringRef k = CFStringCreateWithCString(NULL, key, kCFStringEncodingUTF8);
    if (k) {
        os_unfair_lock_lock(&gFwdLock);
        if (!gSeen) gSeen = CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks, NULL);
        count = (long)(intptr_t)CFDictionaryGetValue(gSeen, k) + 1;
        CFDictionarySetValue(gSeen, k, (const void *)(intptr_t)count);
        os_unfair_lock_unlock(&gFwdLock);
        CFRelease(k);
    }
    if (count <= 3 || (count % 1000) == 0) {
        extern const char *nvmtl_log_path(void);
        FILE *lf = fopen(nvmtl_log_path(), "a");
        const char *rt_ = [[inv methodSignature] methodReturnType];
        static const char *const kApple_[] = { "_MTLFunctionInternal", "_MTLFunction", "_MTLLibrary",
                                               "_MTLDevice", "_MTLCommandQueue", "_MTLCommandBuffer", NULL };
        for (int ci_ = 0; kApple_[ci_]; ci_++) {
            Class ac_ = objc_getClass(kApple_[ci_]);
            Method am_ = ac_ ? class_getInstanceMethod(ac_, [inv selector]) : NULL;
            const char *te_ = am_ ? method_getTypeEncoding(am_) : NULL;
            if (te_) { rt_ = te_; break; }
        }
        int deref_ = rt_ && (rt_[0] == '^' || (rt_[0] == 'r' && rt_[1] == '^'));
        if (lf) { fprintf(lf, "pid %d NVMTL: SPI-STUB -[%s %s]  (call #%ld, zero-returned — NOT IMPLEMENTED)%s\n", getpid(), cn, sn, count,
                          deref_ ? "  POINTER RETURN — a caller that dereferences this NULL will crash" : ""); fclose(lf); }
    }

    NSUInteger len = [[inv methodSignature] methodReturnLength];
    if (len) {
        void *z = calloc(1, len);
        if (z) { [inv setReturnValue:z]; free(z); }
    }
}

static const char *const kNVFwdClasses[] = {
    "NVMTLDevice", "NVMTLCommandQueue", "NVMTLCommandBuffer", "NVMTLBuffer", "NVMTLTexture",
    "NVMTLLibrary", "NVMTLFunction", "NVMTLArgumentEncoder", "NVMTLRenderPipelineState",
    "NVMTLRenderCommandEncoder", "NVMTLBlitCommandEncoder", "NVMTLComputeCommandEncoder",
    "NVMTLAccelerationStructureCommandEncoder", "NVMTLAccelerationStructure",
    "NVMTLComputePipelineState", "NVMTLDepthStencilState", "NVMTLSamplerState", "NVMTLFence",
    "NVMTLHeap", "NVMTLIndirectCommand", "NVMTLIndirectCommandBuffer",
    "NVMTLParallelRenderCommandEncoder", "NVMTLRenderPipelineReflection", "NVMTLComputePipelineReflection", "NVMTLArgument",
    "NVMTLStructType", "NVMTLStructMember", "NVMTLPointerType", "NVMTLTextureReferenceType", "NVMTLVertexAttribute",
    NULL
};

@interface NVMTLForwardInstaller : NSObject @end
@implementation NVMTLForwardInstaller
+ (void)load
{
    gEncCache = CFDictionaryCreateMutable(NULL, 0, &kCFTypeDictionaryKeyCallBacks,
                                                   &kCFTypeDictionaryValueCallBacks);
    int installed = 0;
    for (int i = 0; kNVFwdClasses[i]; i++) {
        Class c = objc_getClass(kNVFwdClasses[i]);
        if (!c) { nvlog("NVMTLForward: class %s not found — NOT protected", kNVFwdClasses[i]); continue; }

        if (gNOrig < (int)(sizeof gOrig / sizeof gOrig[0])) {
            nvfwd_msfs_fn prev = (nvfwd_msfs_fn)class_getMethodImplementation(
                                     c, @selector(methodSignatureForSelector:));
            if (prev == (nvfwd_msfs_fn)nvfwd_msfs) prev = NULL;
            gOrig[gNOrig].cls  = c;
            gOrig[gNOrig].orig = prev;
            gNOrig++;
        }
        class_replaceMethod(c, @selector(methodSignatureForSelector:), (IMP)nvfwd_msfs,    "@@::");
        class_replaceMethod(c, @selector(forwardInvocation:),          (IMP)nvfwd_forward, "v@:@");
        installed++;
    }
    nvlog("NVMTLForward: forwarding net installed on %d classes (was: log-then-abort on 20)", installed);
}
@end
