// 1401
// Copyright (c) 2026 NullMoth Systems. SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0

import Cocoa
import CryptoKit
import IOKit
import Metal
import WebKit

struct Package {
    static let version = "1.0.0"
    static let name = "nullmoth-nvidia-1.0.0.tar.gz"
    static let url = URL(string: "https://github.com/nullmoth/nvidia-macos-driver/releases/download/v1.0.0/nullmoth-nvidia-1.0.0.tar.gz")!
    static let sha256 = "09c4ba6f852b269b8cc77c20f13e185cbcc584532b4fcba8e10d39d1c95e1b6a"
}
let uploadPage = URL(string: "https://nullmothsystems.com/#send")!
let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("NullMoth")
let logs = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask)[0].appendingPathComponent("Logs/NullMoth")

func sh(_ path: String, _ args: [String]) -> String {
    let p = Process(); p.executableURL = URL(fileURLWithPath: path); p.arguments = args
    let o = Pipe(); p.standardOutput = o; p.standardError = Pipe()
    do { try p.run() } catch { return "" }
    let d = o.fileHandleForReading.readDataToEndOfFile(); p.waitUntilExit()
    return String(decoding: d, as: UTF8.self)
}
func sysctl(_ k: String) -> String { sh("/usr/sbin/sysctl", ["-n", k]).trimmingCharacters(in: .whitespacesAndNewlines) }
func sha256(_ url: URL) -> String? {
    guard let h = try? FileHandle(forReadingFrom: url) else { return nil }
    defer { try? h.close() }
    var hasher = SHA256()
    while let d = try? h.read(upToCount: 1 << 20), !d.isEmpty { hasher.update(data: d) }
    return hasher.finalize().map { String(format: "%02x", $0) }.joined()
}
func json(_ o: Any) -> String {
    (try? JSONSerialization.data(withJSONObject: o, options: [.prettyPrinted, .sortedKeys])).map { String(decoding: $0, as: UTF8.self) } ?? "{}"
}

func prop(_ e: io_registry_entry_t, _ k: String) -> Any? {
    IORegistryEntryCreateCFProperty(e, k as CFString, kCFAllocatorDefault, 0)?.takeRetainedValue()
}
func u32(_ e: io_registry_entry_t, _ k: String) -> UInt32? {
    guard let d = prop(e, k) as? Data, d.count >= 4 else { return (prop(e, k) as? NSNumber)?.uint32Value }
    return d.withUnsafeBytes { $0.load(as: UInt32.self) }
}
func str(_ e: io_registry_entry_t, _ k: String) -> String? {
    if let s = prop(e, k) as? String { return s }
    if let d = prop(e, k) as? Data { return String(decoding: d.prefix { $0 != 0 }, as: UTF8.self) }
    return nil
}
func regName(_ e: io_registry_entry_t) -> String {
    var n = [CChar](repeating: 0, count: 128); IORegistryEntryGetName(e, &n); return String(cString: n)
}
func className(_ e: io_registry_entry_t) -> String {
    var n = [CChar](repeating: 0, count: 128); IOObjectGetClass(e, &n); return String(cString: n)
}
func children(_ e: io_registry_entry_t) -> [io_registry_entry_t] {
    var it: io_iterator_t = 0, out: [io_registry_entry_t] = []
    guard IORegistryEntryGetChildIterator(e, kIOServicePlane, &it) == KERN_SUCCESS else { return out }
    var c = IOIteratorNext(it); while c != 0 { out.append(c); c = IOIteratorNext(it) }
    IOObjectRelease(it); return out
}
func services(_ cls: String) -> [io_registry_entry_t] {
    var it: io_iterator_t = 0, out: [io_registry_entry_t] = []
    guard IOServiceGetMatchingServices(kIOMainPortDefault, IOServiceMatching(cls), &it) == KERN_SUCCESS else { return out }
    var s = IOIteratorNext(it); while s != 0 { out.append(s); s = IOIteratorNext(it) }
    IOObjectRelease(it); return out
}

func pciDisplays() -> [[String: Any]] {
    services("IOPCIDevice").compactMap { s in
        defer { IOObjectRelease(s) }
        guard let cls = u32(s, "class-code"), cls >> 16 == 0x03, let v = u32(s, "vendor-id"), let d = u32(s, "device-id") else { return nil }
        return ["vendor": String(format: "%04X", v & 0xffff), "device": String(format: "%04X", d & 0xffff), "model": str(s, "model") ?? ""]
    }
}

func usbControllers() -> [[String: Any]] {
    var out: [[String: Any]] = []
    for x in services("AppleUSBXHCI") {
        var parent: io_registry_entry_t = 0
        IORegistryEntryGetParentEntry(x, kIOServicePlane, &parent)
        let ven = u32(parent, "vendor-id").map { $0 & 0xffff } ?? 0, dev = u32(parent, "device-id").map { $0 & 0xffff } ?? 0
        var path = [CChar](repeating: 0, count: 512)
        IORegistryEntryGetPath(parent, kIOServicePlane, &path)
        var ports: [[String: Any]] = []
        for p in children(x) {
            let cls = className(p)
            if cls.hasPrefix("AppleUSB"), cls.hasSuffix("XHCIPort") {
                let devs = children(p).filter { className($0).contains("USB") && className($0).contains("Device") || className($0) == "IOUSBHostDevice" }
                let names = devs.map { str($0, "USB Product Name") ?? str($0, "kUSBProductString") ?? regName($0) }
                ports.append(["name": regName(p), "port": Int(u32(p, "port") ?? 0), "usb3": cls.contains("30"),
                              "connector": Int(u32(p, "UsbConnector") ?? 255), "devices": names,
                              "comment": str(p, "#comment") ?? ""])
                devs.forEach { IOObjectRelease($0) }
            }
            IOObjectRelease(p)
        }
        out.append(["controller": regName(parent), "vendor": String(format: "%04X", ven), "device": String(format: "%04X", dev),
                    "path": String(cString: path), "ports": ports.sorted { ($0["port"] as! Int) < ($1["port"] as! Int) }])
        IOObjectRelease(parent); IOObjectRelease(x)
    }
    return out
}

func utbMap(_ ctrls: [[String: Any]], _ sel: [String: [String: Int]]) -> ([String: Any]?, String?) {
    var pers: [String: Any] = [:]
    var ids = Set<String>()
    for c in ctrls {
        let name = c["controller"] as! String
        guard let chosen = sel[name], !chosen.isEmpty else { continue }
        if chosen.count > 15 { return (nil, "\(name) has \(chosen.count) ports picked; macOS allows 15 per controller. Untick some.") }
        let id = "0x\(c["device"]!)\(c["vendor"]!)"
        if ids.contains(id) { return (nil, "Two USB controllers share the PCI id \(id); this mapper cannot tell them apart yet.") }
        ids.insert(id)
        var ports: [String: Any] = [:]
        for p in c["ports"] as! [[String: Any]] {
            let pn = p["name"] as! String
            guard let conn = chosen[pn] else { continue }
            var n = UInt32(p["port"] as! Int).littleEndian
            ports[pn] = ["port": Data(bytes: &n, count: 4), "UsbConnector": conn,
                         "#comment": ((p["devices"] as? [String]) ?? []).joined(separator: ", ")]
        }
        pers[name] = ["CFBundleIdentifier": "com.dhinakg.USBToolBox.kext", "IOClass": "USBToolBox", "IOMatchCategory": "USBToolBox",
                      "IOPCIPrimaryMatch": id, "IOProviderClass": "IOPCIDevice",
                      "IOProviderMergeProperties": ["ports": ports, "port-count": Data(bytes: [UInt8(ports.count), 0, 0, 0], count: 4)]]
    }
    if pers.isEmpty { return (nil, "No ports picked.") }
    return (["CFBundleDevelopmentRegion": "English", "CFBundleIdentifier": "com.nullmoth.UTBMap", "CFBundleInfoDictionaryVersion": "6.0",
             "CFBundleName": "UTBMap", "CFBundlePackageType": "KEXT", "CFBundleShortVersionString": "1.0", "CFBundleSignature": "????",
             "CFBundleVersion": "1.0", "IOKitPersonalities": pers, "OSBundleLibraries": ["com.dhinakg.USBToolBox.kext": "1.0.0"],
             "OSBundleRequired": "Root"], nil)
}

let crashDirs = ["/Library/Logs/DiagnosticReports", NSHomeDirectory() + "/Library/Logs/DiagnosticReports"]
let seenFile = support.appendingPathComponent("crash-seen.json")

let driverImages = ["NVMTLDriver", "libnvmtl_translate", "libvulkan_nouveau", "NVIDIAShared"]
func involvesDriver(_ u: URL) -> Bool {
    guard let t = try? String(contentsOf: u, encoding: .utf8) else { return false }
    let body = t.split(separator: "\n", maxSplits: 1).last.map(String.init) ?? t
    if let j = try? JSONSerialization.jsonObject(with: Data(body.utf8)) as? [String: Any] {
        if let ps = j["panicString"] as? String { return panicNamesDriver(ps) }
        guard let fi = j["faultingThread"] as? Int, let th = j["threads"] as? [[String: Any]], fi < th.count,
              let fr = th[fi]["frames"] as? [[String: Any]], let imgs = j["usedImages"] as? [[String: Any]] else { return false }
        return fr.prefix(30).contains { f in
            let ii = f["imageIndex"] as? Int ?? -1
            let n = ii >= 0 && ii < imgs.count ? (imgs[ii]["name"] as? String ?? "") : ""
            return driverImages.contains { n.contains($0) }
        }
    }
    return panicNamesDriver(t)
}
func panicNamesDriver(_ p: String) -> Bool {
    guard let r = p.range(of: "Kernel Extensions in backtrace") else { return false }
    let tail = p[r.upperBound...].prefix(4000)
    let block = tail.components(separatedBy: "\n\n").first ?? String(tail)
    return block.contains("com.nullmoth.")
}

func driverCrashes(sinceInstallOnly: Bool = true) -> [URL] {
    let fm = FileManager.default
    let installed = (try? fm.attributesOfItem(atPath: "/Library/NullMoth/state"))?[.modificationDate] as? Date
    var out: [URL] = []
    for d in crashDirs {
        for n in (try? fm.contentsOfDirectory(atPath: d)) ?? [] where !n.hasPrefix(".") && (n.hasSuffix(".panic") || n.hasSuffix(".ips")) {
            let u = URL(fileURLWithPath: d).appendingPathComponent(n)
            let m = (try? fm.attributesOfItem(atPath: u.path))?[.modificationDate] as? Date ?? .distantPast
            if sinceInstallOnly, let installed, m < installed { continue }
            if involvesDriver(u) { out.append(u) }
        }
    }
    return out.sorted { $0.lastPathComponent < $1.lastPathComponent }
}

func redact(_ s: String) -> String {
    var t = s
    let user = NSUserName(), full = NSFullUserName()
    var exact: [String: String] = [:]
    exact[NSHomeDirectory()] = "/Users/user"; exact["/Users/\(user)"] = "/Users/user"
    for k in ["ComputerName", "LocalHostName", "HostName"] {
        let v = sh("/usr/sbin/scutil", ["--get", k]).trimmingCharacters(in: .whitespacesAndNewlines)
        if v.count > 2 { exact[v] = "this-mac" }
    }
    for e in services("IOPlatformExpertDevice") {
        for k in ["IOPlatformSerialNumber", "IOPlatformUUID", "serial-number"] { if let v = str(e, k), v.count > 3 { exact[v] = "[removed]" } }
        IOObjectRelease(e)
    }
    if full.count > 2 { exact[full] = "user" }
    if user.count > 2 { exact[user] = "user" }
    for (k, v) in exact.sorted(by: { $0.key.count > $1.key.count }) { t = t.replacingOccurrences(of: k, with: v) }
    let rules: [(String, String)] = [
        (#"/Users/[^/\s"']+"#, "/Users/user"),
        (#"\b[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}\b"#, "[uuid]"),
        (#"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b"#, "[mac]"),
        (#"\b(?:\d{1,3}\.){3}\d{1,3}\b"#, "[ip]"),
        (#""(crashReporterKey|deviceIdentifierForVendor|sessionID|userID|incident|bootSessionUUID|sleepWakeUUID|serial[A-Za-z]*)"\s*:\s*"[^"]*""#, "\"$1\":\"[removed]\""),
        (#"(?i)(serial number|system serial|hardware uuid|provisioning udid)[^\n]*"#, "$1: [removed]"),
    ]
    for (p, r) in rules { t = t.replacingOccurrences(of: p, with: r, options: .regularExpression) }
    return t
}

func hardwareFacts() -> String {
    let os = ProcessInfo.processInfo.operatingSystemVersion
    let build = sh("/usr/bin/sw_vers", ["-buildVersion"]).trimmingCharacters(in: .whitespacesAndNewlines)
    let gpus = pciDisplays().map { "\($0["vendor"]!):\($0["device"]!) \($0["model"] as? String ?? "")" }.joined(separator: "; ")
    let mem = (UInt64(sysctl("hw.memsize")) ?? 0) >> 30
    let kexts = sh("/usr/bin/kmutil", ["showloaded", "--list-only"]).split(separator: "\n")
        .filter { $0.contains("com.nullmoth.") }.map { String($0.split(separator: " ").last(where: { $0.hasPrefix("com.") }) ?? "") }
    return """
    1401 crash report (made on this Mac; send it only if you want to)
    driver package : \(Package.version)
    macOS          : \(os.majorVersion).\(os.minorVersion).\(os.patchVersion) (\(build))
    CPU            : \(sysctl("machdep.cpu.brand_string")) (\(sysctl("hw.physicalcpu")) cores / \(sysctl("hw.logicalcpu")) threads)
    memory         : \(mem) GB
    Mac model      : \(sysctl("hw.model"))
    graphics       : \(gpus)
    driver kexts   : \(kexts.isEmpty ? "none loaded now" : kexts.joined(separator: ", "))
    OpenCore       : \(sh("/usr/sbin/nvram", ["4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:opencore-version"]).split(separator: "\t").last.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) } ?? "unknown")

    """
}

func failurePart(_ u: URL) -> String {
    guard let t = try? String(contentsOf: u, encoding: .utf8) else { return "" }
    var body = t
    if let j = t.split(separator: "\n", maxSplits: 1).last.flatMap({ try? JSONSerialization.jsonObject(with: Data($0.utf8)) as? [String: Any] }) {
        if let ps = j["panicString"] as? String { body = ps }
        else {
            var parts: [String] = []
            if let ex = j["exception"] { parts.append("exception: \(ex)") }
            if let te = j["termination"] { parts.append("termination: \(te)") }
            if let fi = j["faultingThread"] as? Int, let th = j["threads"] as? [[String: Any]], fi < th.count,
               let fr = th[fi]["frames"] as? [[String: Any]], let imgs = j["usedImages"] as? [[String: Any]] {
                parts.append("crashed thread \(fi):")
                for (i, f) in fr.prefix(40).enumerated() {
                    let ii = f["imageIndex"] as? Int ?? -1
                    let img = ii >= 0 && ii < imgs.count ? (imgs[ii]["name"] as? String ?? "?") : "?"
                    parts.append("  \(i) \(img) \(f["symbol"] as? String ?? "") +\(f["imageOffset"] ?? "")")
                }
            }
            body = parts.joined(separator: "\n")
        }
    }
    return String(body.prefix(120_000))
}

func makeReport(_ crashes: [URL]) -> String {
    var r = ""
    for u in crashes.suffix(3) {
        r += "\n=== \(u.lastPathComponent.replacingOccurrences(of: #"-\d{4}-\d{2}-\d{2}-\d{6}"#, with: "", options: .regularExpression)) ===\n"
        r += failurePart(u) + "\n"
    }
    return hardwareFacts() + redact(r)
}

func writeReport(_ crashes: [URL], to dst: URL) -> Bool {
    (try? makeReport(crashes).write(to: dst, atomically: true, encoding: .utf8)) != nil
}

func crashCheck(ui: Bool) -> Int32 {
    let seen = Set((try? JSONSerialization.jsonObject(with: Data(contentsOf: seenFile))) as? [String] ?? [])
    let new = driverCrashes().filter { !seen.contains($0.lastPathComponent) }
    print(json(["new": new.map(\.lastPathComponent)]))
    guard !new.isEmpty else { return 0 }
    try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
    if let d = try? JSONSerialization.data(withJSONObject: Array(seen) + new.map(\.lastPathComponent)) { try? d.write(to: seenFile, options: .atomic) }
    guard ui else { return 0 }
    let app = NSApplication.shared; app.setActivationPolicy(.accessory); app.activate(ignoringOtherApps: true)
    let a = NSAlert()
    a.messageText = "1401 noticed the NVIDIA driver crashed your system."
    a.informativeText = "Would you like to send a report? 1401 makes a text file on your Desktop with your hardware and what failed. It leaves out your name, your Mac's name, serial numbers and addresses. You can read it first, then upload it on the NullMoth site. Nothing is sent unless you upload it."
    a.addButton(withTitle: "Make the report"); a.addButton(withTitle: "Not now")
    if let img = Bundle.main.image(forResource: "moth-mark") { a.icon = img }
    if a.runModal() == .alertFirstButtonReturn {
        let f = DateFormatter(); f.dateFormat = "yyyy-MM-dd-HHmm"
        let dst = FileManager.default.urls(for: .desktopDirectory, in: .userDomainMask)[0].appendingPathComponent("1401-crash-\(f.string(from: Date())).txt")
        if writeReport(new, to: dst) {
            NSWorkspace.shared.activateFileViewerSelecting([dst])
            NSWorkspace.shared.open(uploadPage)
        }
    }
    return 0
}

final class App: NSObject, NSApplicationDelegate, WKScriptMessageHandler, WKUIDelegate, URLSessionDownloadDelegate {
    var window: NSWindow!
    var web: WKWebView!
    var table: [String: Any] = [:]
    var seenUsb: [String: Set<String>] = [:]
    var usbTimer: Timer?

    func applicationDidFinishLaunching(_ n: Notification) {
        let res = Bundle.main.resourceURL!
        if let d = try? Data(contentsOf: res.appendingPathComponent("nvidia_gsp_ids.json")),
           let j = try? JSONSerialization.jsonObject(with: d) as? [String: Any] { table = j }
        let cfg = WKWebViewConfiguration()
        cfg.userContentController.add(self, name: "nm")
        web = WKWebView(frame: .zero, configuration: cfg)
        web.uiDelegate = self
        web.setValue(false, forKey: "drawsBackground")
        window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 880, height: 800),
                          styleMask: [.titled, .closable, .miniaturizable, .resizable], backing: .buffered, defer: false)
        window.title = "1401"; window.backgroundColor = .black; window.contentView = web; window.center()
        window.makeKeyAndOrderFront(nil)
        web.loadFileURL(res.appendingPathComponent("index.html"), allowingReadAccessTo: res)
        NSApp.activate(ignoringOtherApps: true)
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ s: NSApplication) -> Bool { true }

    func webView(_ w: WKWebView, runJavaScriptConfirmPanelWithMessage m: String, initiatedByFrame f: WKFrameInfo, completionHandler: @escaping (Bool) -> Void) {
        let a = NSAlert(); a.messageText = m; a.addButton(withTitle: "OK"); a.addButton(withTitle: "Cancel")
        completionHandler(a.runModal() == .alertFirstButtonReturn)
    }

    func send(_ event: String, _ data: Any) {
        guard let j = try? JSONSerialization.data(withJSONObject: ["event": event, "data": data]),
              let s = String(data: j, encoding: .utf8) else { return }
        DispatchQueue.main.async { self.web.evaluateJavaScript("NM.on(\(s))") }
    }

    func userContentController(_ c: WKUserContentController, didReceive m: WKScriptMessage) {
        guard let b = m.body as? [String: Any], let act = b["act"] as? String else { return }
        switch act {
        case "scan": DispatchQueue.global().async { self.send("scan", self.scan()) }
        case "download": download()
        case "run": run(mode: b["mode"] as? String ?? "dry", pkg: b["pkg"] as? String ?? "", efi: b["efi"] as? String ?? "auto", extra: [])
        case "restart": NSAppleScript(source: "tell application \"System Events\" to restart")?.executeAndReturnError(nil)
        case "privacy": NSWorkspace.shared.open(URL(string: "x-apple.systempreferences:com.apple.preference.security?Security")!)
        case "logs": try? FileManager.default.createDirectory(at: logs, withIntermediateDirectories: true); NSWorkspace.shared.open(logs)
        case "open": if let u = b["url"] as? String, let url = URL(string: u), url.scheme == "https" { NSWorkspace.shared.open(url) }
        case "usbStart": usbWatch(true)
        case "usbStop": usbWatch(false)
        case "usbWrite": usbWrite(b["sel"] as? [String: [String: Int]] ?? [:], efi: b["efi"] as? String ?? "auto")
        case "crashReport": crashReportFromWindow()
        case "verbose": run(mode: "verbose", pkg: "", efi: b["efi"] as? String ?? "auto", extra: ["--verbose", (b["on"] as? Bool ?? false) ? "on" : "off"])
        default: break
        }
    }

    func scan() -> [String: Any] { App.scanMac(table) }
    static func scanMac(_ table: [String: Any]) -> [String: Any] {
        let ids = table["ids"] as? [String: String] ?? [:], tested = table["tested"] as? [String] ?? []
        let gpus: [[String: Any]] = pciDisplays().map { g in
            let nv = g["vendor"] as? String == "10DE", dev = g["device"] as? String ?? ""
            var r = g
            r["name"] = nv ? (ids[dev] ?? "NVIDIA \(dev)") : ((g["model"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? "\(g["vendor"]!):\(dev)")
            r["supported"] = nv && ids[dev] != nil; r["tested"] = nv && tested.contains(dev)
            return r
        }
        let os = ProcessInfo.processInfo.operatingSystemVersion
        let oc = sh("/usr/sbin/nvram", ["4D1FDA02-38C7-4A6A-9CC6-4BCCA8B30102:opencore-version"])
            .split(separator: "\t").dropFirst().first.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) } ?? ""
        let kexts = sh("/usr/bin/kmutil", ["showloaded", "--list-only"]).split(separator: "\n").filter { $0.contains("com.nullmoth.") }.count
        let fm = FileManager.default
        let files = fm.fileExists(atPath: "/Library/GPUBundles/NVMTLDriver.bundle") && fm.fileExists(atPath: "/Library/Extensions/NVRM.kext")
        var arch = "x86_64"
        #if arch(arm64)
        arch = "arm64"
        #endif
        let safe = sh("/usr/sbin/nvram", ["boot-args"]).split(whereSeparator: { $0 == " " || $0 == "\t" || $0 == "\n" }).contains("-nvoff")
        return ["macos": "\(os.majorVersion).\(os.minorVersion).\(os.patchVersion)", "major": os.majorVersion, "arch": arch,
                "gpus": gpus, "opencore": oc, "kexts": kexts, "files": files, "metal": MTLCopyAllDevices().map { $0.name },
                "packages": findPackages(), "version": Package.version, "safemode": safe,
                "record": fm.fileExists(atPath: "/Library/NullMoth/state"), "crashes": driverCrashes().map(\.lastPathComponent),
                "translated": sysctl("sysctl.proc_translated") == "1"]
    }

    static func findPackages() -> [[String: String]] {
        let fm = FileManager.default
        var c = ((try? fm.contentsOfDirectory(atPath: "/Volumes")) ?? []).map { URL(fileURLWithPath: "/Volumes/\($0)/NullMoth/\(Package.name)") }
        c.append(fm.urls(for: .downloadsDirectory, in: .userDomainMask)[0].appendingPathComponent(Package.name))
        c.append(support.appendingPathComponent(Package.name))
        return c.filter { fm.fileExists(atPath: $0.path) }.map { u in
            ["path": u.path, "where": u.path.hasPrefix("/Volumes/") ? "the 1401 stick or disk image" : u.path.contains("/Downloads/") ? "Downloads" : "an earlier download",
             "ok": sha256(u) == Package.sha256 ? "yes" : "no"]
        }
    }

    func download() {
        try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
        URLSession(configuration: .default, delegate: self, delegateQueue: nil).downloadTask(with: Package.url).resume()
        send("dl", ["state": "start"])
    }
    func urlSession(_ s: URLSession, downloadTask t: URLSessionDownloadTask, didWriteData b: Int64, totalBytesWritten w: Int64, totalBytesExpectedToWrite e: Int64) {
        send("dl", ["state": "progress", "done": w, "total": e])
    }
    func urlSession(_ s: URLSession, downloadTask t: URLSessionDownloadTask, didFinishDownloadingTo loc: URL) {
        let code = (t.response as? HTTPURLResponse)?.statusCode ?? 0
        let dst = support.appendingPathComponent(Package.name)
        guard code == 200 else { send("dl", ["state": "error", "why": "the server answered HTTP \(code)"]); return }
        guard sha256(loc) == Package.sha256 else { send("dl", ["state": "error", "why": "the download does not match its SHA-256, so it was thrown away"]); return }
        try? FileManager.default.removeItem(at: dst)
        do { try FileManager.default.moveItem(at: loc, to: dst) } catch { send("dl", ["state": "error", "why": error.localizedDescription]); return }
        send("dl", ["state": "done", "path": dst.path])
    }
    func urlSession(_ s: URLSession, task: URLSessionTask, didCompleteWithError e: Error?) {
        if let e { send("dl", ["state": "error", "why": e.localizedDescription]) }
    }

    func usbWatch(_ on: Bool) {
        usbTimer?.invalidate(); usbTimer = nil
        if on {
            seenUsb = [:]
            usbTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { _ in self.usbTick() }
            usbTick()
        }
    }
    func usbTick() {
        DispatchQueue.global().async {
            let c = usbControllers()
            for ctl in c {
                let n = ctl["controller"] as! String
                for p in ctl["ports"] as! [[String: Any]] where !((p["devices"] as? [String]) ?? []).isEmpty {
                    self.seenUsb[n, default: []].insert(p["name"] as! String)
                }
            }
            self.send("usb", ["controllers": c, "seen": self.seenUsb.mapValues { Array($0) }])
        }
    }
    func usbWrite(_ sel: [String: [String: Int]], efi: String) {
        let (plist, err) = utbMap(usbControllers(), sel)
        guard let plist else { send("usbErr", err ?? "could not build the map"); return }
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("nullmoth-utb-\(UUID().uuidString)/UTBMap.kext/Contents")
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        guard let d = try? PropertyListSerialization.data(fromPropertyList: plist, format: .xml, options: 0),
              (try? d.write(to: dir.appendingPathComponent("Info.plist"))) != nil else { send("usbErr", "could not write the map"); return }
        run(mode: "usbmap", pkg: "", efi: efi, extra: ["--usbmap", dir.deletingLastPathComponent().path])
    }

    func crashReportFromWindow() {
        let cr = driverCrashes(sinceInstallOnly: false)
        guard !cr.isEmpty else { send("crashDone", ["ok": false, "why": "No crash that names the driver was found."]); return }
        let f = DateFormatter(); f.dateFormat = "yyyy-MM-dd-HHmm"
        let dst = FileManager.default.urls(for: .desktopDirectory, in: .userDomainMask)[0].appendingPathComponent("1401-crash-\(f.string(from: Date())).txt")
        if writeReport(cr, to: dst) {
            NSWorkspace.shared.activateFileViewerSelecting([dst]); NSWorkspace.shared.open(uploadPage)
            send("crashDone", ["ok": true, "path": dst.path])
        } else { send("crashDone", ["ok": false, "why": "Could not write the report to the Desktop."]) }
    }

    func run(mode: String, pkg: String, efi: String, extra: [String]) {
        try? FileManager.default.createDirectory(at: logs, withIntermediateDirectories: true)
        let stamp = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "")
        let log = logs.appendingPathComponent("setup-\(mode)-\(stamp).log")
        FileManager.default.createFile(atPath: log.path, contents: nil)
        let res = Bundle.main.resourceURL!
        var args: [String]
        switch mode {
        case "remove": args = ["--remove"]
        case "usbmap", "verbose": args = extra
        default:
            args = ["--pkg", pkg, "--sha", Package.sha256, "--tool", res.appendingPathComponent("NullMothSafe.efi").path,
                    "--app", Bundle.main.executablePath ?? ""]
            if mode == "dry" { args.append("--dry") }
        }
        if mode != "remove", efi != "auto", !efi.isEmpty { args += ["--efi", efi] }
        let q = { (s: String) in "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'" }
        let cmd = "/bin/bash \(q(res.appendingPathComponent("nullmoth-setup.sh").path)) \(args.map(q).joined(separator: " ")) >> \(q(log.path)) 2>&1"
        let asrc = "do shell script \"\(cmd.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\""))\" with administrator privileges"
        send("run", ["state": "start", "mode": mode, "log": log.path])
        DispatchQueue.global().async {
            let done = DispatchSemaphore(value: 0)
            var sent = 0
            DispatchQueue.global().async {
                while done.wait(timeout: .now() + 0.3) == .timedOut { sent = self.flush(log, from: sent) }
            }
            var err: NSDictionary?
            NSAppleScript(source: asrc)?.executeAndReturnError(&err)
            done.signal()
            Thread.sleep(forTimeInterval: 0.4)
            sent = self.flush(log, from: sent)
            if let err, (err[NSAppleScript.errorNumber] as? Int) == -128 { self.send("run", ["state": "cancelled", "mode": mode]); return }
            let text = (try? String(contentsOf: log, encoding: .utf8)) ?? ""
            self.send("run", ["state": "end", "mode": mode, "ok": text.contains("RESULT ok"), "log": log.path])
        }
    }
    func flush(_ log: URL, from: Int) -> Int {
        guard let t = try? String(contentsOf: log, encoding: .utf8) else { return from }
        let lines = t.split(separator: "\n", omittingEmptySubsequences: false).dropLast()
        if lines.count > from { send("lines", Array(lines[from...]).map(String.init)) }
        return max(from, lines.count)
    }
}

let argv = CommandLine.arguments
func table() -> [String: Any] {
    guard let d = try? Data(contentsOf: Bundle.main.resourceURL!.appendingPathComponent("nvidia_gsp_ids.json")) else { return [:] }
    return (try? JSONSerialization.jsonObject(with: d) as? [String: Any]) ?? [:]
}
if argv.contains("--scan") { print(json(App.scanMac(table()))); exit(0) }
if argv.contains("--usb") { print(json(usbControllers())); exit(0) }
if let i = argv.firstIndex(of: "--crash-report"), i + 1 < argv.count {
    let cr = driverCrashes(sinceInstallOnly: false)
    let ok = !cr.isEmpty && writeReport(cr, to: URL(fileURLWithPath: argv[i + 1]))
    print(json(["crashes": cr.map(\.lastPathComponent), "written": ok])); exit(ok ? 0 : 1)
}
if argv.contains("--crash-check") { exit(crashCheck(ui: !argv.contains("--no-ui"))) }
let app = NSApplication.shared
let delegate = App()
app.delegate = delegate
app.setActivationPolicy(.regular)
app.run()
