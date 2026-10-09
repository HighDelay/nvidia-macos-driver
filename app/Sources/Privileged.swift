import Foundation

/// Runs a shell command as root through the macOS administrator password prompt, OFF the main thread.
///
/// WAS: NSAppleScript. It is not thread-safe, so Send logs ran it on the main thread, and the whole app froze for as
/// long as log collection took (minutes, with tens of MB of output pulled back through the AppleScript result); users
/// force-quit it. Setup ran it on a background queue instead, where its password panel could hang unseen.
/// /usr/bin/osascript is its own process: the password panel works from any thread, the app keeps drawing, and the
/// command's output goes to a file instead of through the AppleScript result.
enum Privileged {
    struct Result {
        let output: String       // everything the command printed (stdout and stderr)
        let error: String?       // why it did not run or did not finish; nil when it ran
        let cancelled: Bool      // the user pressed Cancel at the password prompt
    }

    static func run(_ command: String, timeout: TimeInterval, admin: Bool = true) -> Result {
        let fm = FileManager.default
        let dir = fm.temporaryDirectory.appendingPathComponent("1401-run-" + UUID().uuidString)
        do { try fm.createDirectory(at: dir, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700]) }
        catch { return Result(output: "", error: "Could not make a temporary folder: \(error.localizedDescription)", cancelled: false) }
        defer { try? fm.removeItem(at: dir) }
        let out = dir.appendingPathComponent("output.txt")
        let q = { (s: String) in "'" + s.replacingOccurrences(of: "'", with: "'\\''") + "'" }
        // root writes the output, then hands the file back to this user so the app can read and delete it
        let shell = "{ \(command) ; } > \(q(out.path)) 2>&1; /usr/sbin/chown \(getuid()) \(q(out.path)); exit 0"
        let source = "do shell script \"" + shell.replacingOccurrences(of: "\\", with: "\\\\").replacingOccurrences(of: "\"", with: "\\\"") + (admin ? "\" with administrator privileges" : "\"")
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        p.arguments = ["-e", source]
        let err = Pipe()
        p.standardError = err
        p.standardOutput = FileHandle.nullDevice
        let finished = DispatchSemaphore(value: 0)
        p.terminationHandler = { _ in finished.signal() }
        do { try p.run() } catch {
            return Result(output: "", error: "Could not start the password prompt: \(error.localizedDescription)", cancelled: false)
        }
        let timedOut = finished.wait(timeout: .now() + timeout) == .timedOut
        if timedOut { p.terminate(); _ = finished.wait(timeout: .now() + 5) }
        let stderr = String(decoding: err.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
        let output = (try? String(contentsOf: out, encoding: .utf8)) ?? ""
        if timedOut { return Result(output: output, error: "Stopped after \(Int(timeout / 60)) minutes without finishing.", cancelled: false) }
        // osascript reports Cancel at the password prompt as error -128 ("User canceled.")
        let cancelled = stderr.contains("-128")
        if p.terminationStatus != 0 {
            return Result(output: output, error: cancelled ? "Cancelled at the password prompt." : stderr.trimmingCharacters(in: .whitespacesAndNewlines), cancelled: cancelled)
        }
        return Result(output: output, error: nil, cancelled: false)
    }
}
