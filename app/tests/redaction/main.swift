import Foundation

let technical = "com.apple.driver.AppleIntelFramebuffer AppleIntel AppleACPIPlatform macOS machdep.cpu.vendor 10DE:2D04 8086:A78B ACPI _SB.PCI0.GP17 std::vector"
for account in ["apple", "mac", "intel"] {
    assert(redactSupportText(technical, homes: [], identities: [account: "user"]) == technical)
}
let home = "/" + ["Users", "ExampleAccount"].joined(separator: "/")
let raw = "account EXAMPLEACCOUNT owner Example Person host Example-Computer " + home + "/Library/log.txt"
let result = redactSupportText(raw, homes: [home], identities: ["ExampleAccount": "user", "Example Person": "user", "Example-Computer": "this-mac"])
assert(!result.localizedCaseInsensitiveContains("ExampleAccount"))
assert(!result.localizedCaseInsensitiveContains("Example Person"))
assert(!result.localizedCaseInsensitiveContains("Example-Computer"))
assert(result.contains("[home]/Library/log.txt"))
let boundaries = redactSupportText(home + "Suffix/file " + home.uppercased() + "/file", homes: [home], identities: [:])
assert(!boundaries.localizedCaseInsensitiveContains("ExampleAccount"))
let addresses = "2001:db8::1234 fe80::abcd%en0 ::ffff:192.0.2.4 [2001:db8::2]:443 192.0.2.8 aa:bb:cc:dd:ee:ff"
let safeAddresses = redactSupportText(addresses, homes: [], identities: [:])
assert(safeAddresses == "[ip] [ip] [ip] [[ip]]:443 [ip] [mac]")
let keys = redactSupportText("{\"USERID\":\"secret\",\"SerialNumber\":\"secret\"}\nSystem Serial: secret", homes: [], identities: [:])
assert(!keys.contains("secret"))
// Negative control demonstrates why raw substring replacement is unsafe.
assert(technical.replacingOccurrences(of: "apple", with: "user") != technical)
print("Full identity/path/address redaction preserves PCI/ACPI, framework and CPU symbols.")
