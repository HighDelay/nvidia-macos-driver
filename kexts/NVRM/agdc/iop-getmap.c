/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#include <CoreFoundation/CoreFoundation.h>
#include <dlfcn.h>
#include <stdio.h>
int main(void) {
    void *h = dlopen("/System/Library/PrivateFrameworks/IOPresentment.framework/IOPresentment", RTLD_NOW);
    if (!h) { printf("UNKNOWN  dlopen IOPresentment: %s\n", dlerror()); return 2; }
    CFTypeRef (*getmap)(void) = (CFTypeRef (*)(void))dlsym(h, "IOPresentmentGetMap");
    if (!getmap) { printf("UNKNOWN  no IOPresentmentGetMap symbol\n"); return 2; }
    CFTypeRef m = getmap();
    if (!m) { printf("MAP NULL\n"); return 1; }
    CFStringRef d = CFCopyDescription(m); char buf[4096] = { 0 };
    if (d) CFStringGetCString(d, buf, sizeof buf, kCFStringEncodingUTF8);
    printf("MAP PRESENT  typeid %lu\n%s\n", CFGetTypeID(m), buf); return 0;
}
