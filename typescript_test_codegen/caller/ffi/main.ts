import { SyncChildProcess } from "sync-child-process";
import { Bridge } from "./bridge.ts";
import { MessageSocket, SynchronousSocket } from "./socket.ts";
import { runtimeEnvironment } from "./util.ts";

let internal_bridge: Bridge;

const environment = runtimeEnvironment();

export function establishBridge(): Bridge {
    if (internal_bridge) return internal_bridge;

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
    // TODO: provide lifecycle management to terminate the library process
    const library = new SyncChildProcess("/bin/sh", ["-c", invoke]);
    let stdoutText = "";
    while (true) {
        const nextEvent = library.next();
        if (nextEvent.done) {
            throw new Error(`Library process exited before printing the socket path: ${nextEvent.value?.code}`);
        }
        if (nextEvent.value.type === "stdout") {
            const buffer = nextEvent.value.data;
            stdoutText = `${stdoutText}${buffer.toString("utf-8")}`;
            if (stdoutText.includes("\n")) {
                break;
            }
        }
    }

    // Be robust if the command prints extra lines: take the last non-empty line as the path.
    const socketPath = stdoutText
        .trim()
        .split(/\r?\n/)
        .map((l) => l.trim())
        .filter((l) => l.length > 0)
        .at(-1);

    if (!socketPath) {
        throw new Error(
            "Library process did not print a socket path to stdout",
        );
    }

    const streamSocket = SynchronousSocket.fromPath(socketPath);
    const datagramSocket = new MessageSocket(streamSocket);
    internal_bridge = new Bridge(datagramSocket);
    return internal_bridge;
}
