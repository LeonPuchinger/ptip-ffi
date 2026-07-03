export function runtimeEnvironment(): "node" | "deno" | "unknown" {
    if (typeof Deno !== "undefined" && Deno?.version?.deno) {
        return "deno";
    }

    if (typeof process !== "undefined" && process?.versions?.node) {
        return "node";
    }

    return "unknown";
}
