#![no_main]
#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use core::time::Duration;
use uefi::prelude::*;
use uefi::runtime::{self, VariableAttributes, VariableVendor};
use uefi::{cstr16, guid, println};

const APPLE_BOOT: VariableVendor = VariableVendor(guid!("7c436110-ab2a-4bbb-a880-fe41995c9f82"));
const SAFE: &[u8] = b"-nvoff";

fn has_safe(args: &[u8]) -> bool {
    args.split(|&b| b == b' ' || b == 0).any(|w| w == SAFE)
}

fn fail(msg: &str, st: Status) -> Status {
    println!("NullMoth: {} ({:?}). Nothing was changed.", msg, st);
    println!("Nothing was changed. Choose macOS to start normally.");
    boot::stall(Duration::from_secs(8));
    Status::ABORTED
}

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    let name = cstr16!("boot-args");
    let mut buf = [0u8; 1024];
    let current: Vec<u8> = match runtime::get_variable(name, &APPLE_BOOT, &mut buf) {
        Ok((v, _)) => v.iter().copied().take_while(|&b| b != 0).collect(),
        Err(e) if e.status() == Status::NOT_FOUND => Vec::new(),
        Err(e) => return fail("could not read boot-args", e.status()),
    };
    if has_safe(&current) {
        println!("NullMoth: the next macOS start already has the NVIDIA driver off.");
    } else {
        let mut next = current.clone();
        if !next.is_empty() {
            next.push(b' ');
        }
        next.extend_from_slice(SAFE);
        let attrs = VariableAttributes::NON_VOLATILE | VariableAttributes::BOOTSERVICE_ACCESS | VariableAttributes::RUNTIME_ACCESS;
        if let Err(e) = runtime::set_variable(name, &APPLE_BOOT, attrs, &next) {
            return fail("could not write boot-args", e.status());
        }
        let mut chk = [0u8; 1024];
        if !matches!(runtime::get_variable(name, &APPLE_BOOT, &mut chk), Ok((v, _)) if has_safe(v)) {
            return fail("the firmware did not keep the change", Status::DEVICE_ERROR);
        }
    }
    let attrs = VariableAttributes::NON_VOLATILE | VariableAttributes::BOOTSERVICE_ACCESS | VariableAttributes::RUNTIME_ACCESS;
    if let Err(e) = runtime::set_variable(cstr16!("nullmoth-remove"), &APPLE_BOOT, attrs, b"1") {
        return fail("could not set the remove flag", e.status());
    }
    println!("");
    println!("  NullMoth: the next macOS start removes the NVIDIA driver and restarts by itself.");
    println!("  Choose your macOS disk now, or restart: the driver stays off until it has been removed.");
    println!("  The screen may stay dark until the second restart.");
    println!("  Afterwards this Mac is back to how it was before NullMoth was installed.");
    boot::stall(Duration::from_secs(6));
    Status::SUCCESS
}
