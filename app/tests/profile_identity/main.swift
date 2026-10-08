import Foundation

let rules: [String: Any] = ["rules": [
    ["id": "board-rule", "match": ["chipset": ["B650"]], "conf": ["NVRM_TEST": "1"]],
    ["id": "live-rule", "match": ["arch": ["turing"]], "conf": ["NVMTL_TEST": "1"]]
]]
let original: [String: Any] = ["arch": "turing", "cpu_vendor": "amd", "cpu": "same processor", "gpu_id": "same device"]
let first: [String: Any] = ["chipset": "B650", "board": "BoardOne", "cpu": "same processor", "gpu_id": "same device"]
let second: [String: Any] = ["chipset": "X570", "board": "BoardTwo", "cpu": "same processor", "gpu_id": "same device"]
for candidates in [[first], [first, second], [second, first]] {
    var profile = original
    Profile.attachWindowsDiagnostics(candidates, to: &profile)
    assert(profile["chipset"] == nil)
    assert(profile["windows_profile_status"] as? String == "unverified")
    assert((profile["windows_candidates"] as? [[String: Any]])?.count == candidates.count)
    let selected = Profile.select(rules, profile)
    assert(selected.ids == ["live-rule"])
    assert(selected.conf["NVRM_TEST"] == nil)
}
var live = original
live["chipset"] = "B650"
Profile.attachWindowsDiagnostics([second], to: &live)
assert(live["chipset"] as? String == "B650")
assert(Profile.select(rules, live).ids == ["board-rule", "live-rule"])
var noProfile = original
Profile.attachWindowsDiagnostics([], to: &noProfile)
assert(noProfile["windows_candidates"] == nil)
print("Unbound mounted profiles cannot select chipset rules; live rules and all diagnostics preserved.")
