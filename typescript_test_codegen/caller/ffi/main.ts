import { Bridge } from "./bridge.ts";
import { MessageSocket, SynchronousSocket } from "./socket.ts";
import { runtimeEnvironment } from "./util.ts";

let internal_bridge: Bridge;
let spawnSync: typeof import("node:child_process").spawnSync | undefined;

// Conditionally load spawnSync at module level via top-level await
const _runtimeEnv = runtimeEnvironment();
if (_runtimeEnv === "node") {
    ({ spawnSync } = await import("node:child_process"));
}

export function establishBridge(): Bridge {
    if (internal_bridge) return internal_bridge;

    const environment = runtimeEnvironment();

    // Read the command from the environment variable.
    const invoke = environment === "deno"
        ? Deno.env.get("FFI_LIBRARY_INVOKE")
        : environment === "node"
            ? process.env.FFI_LIBRARY_INVOKE
            : undefined;

    if (!invoke || invoke.trim() === "") {
        throw new Error("FFI_LIBRARY_INVOKE is not set");
    }

    // Run the command and capture stdout/stderr (expects stdout to contain the socket path).
    let stdoutText = "";
    let stderrText = "";
    let exitCode = 0;

    if (environment === "deno") {
        const decoder = new TextDecoder();
        const cmd = new Deno.Command("/bin/sh", {
            args: ["-c", invoke],
            stdout: "piped",
            stderr: "piped",
        });

        const res = cmd.outputSync();
        stdoutText = decoder.decode(res.stdout);
        stderrText = decoder.decode(res.stderr);
        exitCode = res.code;

        if (!res.success) {
            if (stderrText.trim()) console.error(stderrText);
            throw new Error(
                `Failed to start library process (exit code ${exitCode})`,
            );
        }
    } else if (environment === "node") {
        const result = spawnSync!("/bin/sh", ["-c", invoke], {
            stdio: ["ignore", "pipe", "pipe"],
            encoding: "utf8",
        });

        stdoutText = result.stdout ?? "";
        stderrText = result.stderr ?? "";
        exitCode = result.status ?? 0;

        if (exitCode !== 0) {
            if (stderrText.trim()) console.error(stderrText);
            throw new Error(
                `Failed to start library process (exit code ${exitCode})`,
            );
        }
    } else {
        throw new Error("Runtime environment not supported");
    }

    // Be robust if the command prints extra lines: take the last non-empty line as the path.
    const socketPath = stdoutText
        .trim()
        .split(/\r?\n/)
        .map((l) => l.trim())
        .filter((l) => l.length > 0)
        .at(-1);

    if (!socketPath) {
        if (stderrText.trim()) console.error(stderrText);
        throw new Error(
            "Library process did not print a socket path to stdout",
        );
    }

    const streamSocket = SynchronousSocket.fromPath(socketPath);
    const datagramSocket = new MessageSocket(streamSocket);
    internal_bridge = new Bridge(datagramSocket);
    return internal_bridge;
}
