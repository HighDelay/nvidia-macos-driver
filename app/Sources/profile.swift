import Foundation
import IOKit

/// What decides which per-system rules apply: the NVIDIA card and its generation, the CPU, laptop or desktop, eGPU, the
/// macOS major version, and the real board/chipset when the 1401 stick (which read them on Windows) is plugged in.
enum Profile {
    /// NVIDIA's device-ID ranges per generation (GSP-capable cards only: Turing and later).
    static func arch(_ dev: String) -> String {
        guard let v = Int(dev, radix: 16) else { return "unknown" }
        switch v {
        case 0x1E00...0x21FF: return "turing"
        case 0x2200...0x25FF: return v >= 0x2300 && v < 0x2400 ? "hopper" : "ampere"
        case 0x2600...0x28FF: return "ada"
        case 0x2900...0x2FFF: return "blackwell"
        default: return "unknown"
        }
    }

    /// Does `profile` satisfy every key in `match`? Unknown keys never match (a rule must not apply by accident).
    static func matches(_ match: [String: Any], _ p: [String: Any]) -> Bool {
        for (k, want) in match {
            switch k {
            case "arch", "cpu_vendor", "gpu_id", "chipset":
                guard let list = want as? [String], let have = p[k] as? String, list.contains(where: { $0.caseInsensitiveCompare(have) == .orderedSame }) else { return false }
            case "laptop", "egpu":
                guard let w = want as? Bool, let have = p[k] as? Bool, w == have else { return false }
            case "macos_major":
                guard let list = want as? [Int], let have = p[k] as? Int, list.contains(have) else { return false }
            default:
                return false
            }
        }
        return true
    }

    /// The rules from `rules` (the parsed nullmoth-rules.json) that apply to `p`, and the knobs they set.
    static func select(_ rules: [String: Any], _ p: [String: Any]) -> (ids: [String], conf: [String: String]) {
        var ids: [String] = [], conf: [String: String] = [:]
        for r in (rules["rules"] as? [[String: Any]]) ?? [] {
            guard let id = r["id"] as? String, matches(r["match"] as? [String: Any] ?? [:], p) else { continue }
            ids.append(id)
            for (k, v) in (r["conf"] as? [String: String]) ?? [:]
            where k.range(of: #"^(NVMTL|NVK|NVRM)_[A-Z0-9_]+$"#, options: .regularExpression) != nil
               && v.range(of: #"^[A-Za-z0-9._-]{0,64}$"#, options: .regularExpression) != nil { conf[k] = v }
        }
        return (ids, conf)
    }

    /// True when a Thunderbolt bridge sits between the root complex and the NVIDIA card (an eGPU).
    static func nvidiaBehindThunderbolt() -> Bool {
        var it: io_iterator_t = 0
        guard IOServiceGetMatchingServices(kIOMainPortDefault, IOServiceMatching("IOPCIDevice"), &it) == KERN_SUCCESS else { return false }
        defer { IOObjectRelease(it) }
        var found = false
        while case let dev = IOIteratorNext(it), dev != 0 {
            defer { IOObjectRelease(dev) }
            guard let v = IORegistryEntryCreateCFProperty(dev, "vendor-id" as CFString, kCFAllocatorDefault, 0)?.takeRetainedValue() as? Data,
                  v.count >= 2, v[0] == 0xDE, v[1] == 0x10,
                  let c = IORegistryEntryCreateCFProperty(dev, "class-code" as CFString, kCFAllocatorDefault, 0)?.takeRetainedValue() as? Data,
                  c.count >= 3, c[2] == 0x03 else { continue }
            var cur = dev; IOObjectRetain(cur)
            for _ in 0..<16 {
                var parent: io_registry_entry_t = 0
                guard IORegistryEntryGetParentEntry(cur, kIOServicePlane, &parent) == KERN_SUCCESS else { break }
                IOObjectRelease(cur); cur = parent
                var name = [CChar](repeating: 0, count: 128)
                IOObjectGetClass(cur, &name)
                let cls = String(cString: name)
                if cls.localizedCaseInsensitiveContains("Thunderbolt") { found = true; break }
                if cls == "IOPCIBridge" || cls.hasSuffix("PlatformExpert") { continue }
            }
            IOObjectRelease(cur)
            if found { break }
        }
        return found
    }
}
