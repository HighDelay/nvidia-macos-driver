// The driver stays off when the boot picker's "1401: Remove NVIDIA driver" tool asked for it.
// The tool writes -nvoff to boot-args and sets the NVRAM flag nullmoth-remove. OpenCore rewrites boot-args at every
// start (the install lists it under NVRAM Delete), so -nvoff only reached macOS when the user picked macOS in the same
// picker session; anyone who restarted instead got the driver back and the same failed start. The flag is not rewritten
// and stays set until the recover daemon has removed the driver, so it is read too.
#pragma once
#include <IOKit/IORegistryEntry.h>
#include <IOKit/IODeviceTreeSupport.h>
#include <pexpert/pexpert.h>

static inline bool nvrm_driver_off(void)
{
    char arg[8];
    if (PE_parse_boot_argn("-nvoff", arg, sizeof arg))
        return true;
    IORegistryEntry *options = IORegistryEntry::fromPath("/options", gIODTPlane);
    if (!options)
        return false;  // NVRAM not published yet: the boot-arg alone decides, as before
    OSObject *flag = options->copyProperty("nullmoth-remove");
    options->release();
    if (!flag)
        return false;
    flag->release();
    return true;
}
