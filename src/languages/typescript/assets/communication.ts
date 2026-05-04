/*
Protocol layer for the FFI communication protocol described in `doc/communication.md`.

This module sits on top of a `MessageSocket` transport that exchanges UTF-8 text
messages (already framed as netstrings in `socket.ts`).
*/

import type { MessageSocket } from "./socket.ts";

export type UUID = string;

export interface Message {
    serialize(): string;
}

export type ParameterKind =
    | "integer"
    | "float"
    | "boolean"
    | "string"
    | "reference";

export type ParamValue =
    | { kind: "integer"; value: number }
    | { kind: "float"; value: number }
    | { kind: "boolean"; value: boolean }
    | { kind: "string"; value: string }
    | { kind: "reference"; value: UUID };

export class CallMessage implements Message {
    readonly invocationPath: string;
    readonly returnSink: UUID;
    readonly positionalParameters: ParamValue[];
    readonly namedParameters: Map<string, ParamValue>;

    constructor(args: {
        invocationPath: string;
        returnSink: UUID;
        positional?: ParamValue[];
        named?: Map<string, ParamValue>;
    }) {
        this.invocationPath = args.invocationPath;
        this.returnSink = args.returnSink;
        this.positionalParameters = args.positional ?? [];
        this.namedParameters = args.named ?? new Map();
    }

    serialize(): string {
        assertNoCRLF(this.invocationPath);
        const lines: string[] = [
            "C",
            encodeBase64NoPadUtf8(this.invocationPath),
            this.returnSink,
        ];
        for (const p of this.positionalParameters) {
            lines.push(encodeParameterLine(p));
        }
        for (const [name, value] of this.namedParameters) {
            lines.push(encodeParameterLine(value, name));
        }
        return lines.join("\n");
    }
}

export class RequestMessage implements Message {
    readonly parent: UUID;
    readonly accessor: string;
    readonly valueSink: UUID;

    constructor(args: { parent: UUID; accessor: string; valueSink: UUID }) {
        this.parent = args.parent;
        this.accessor = args.accessor;
        this.valueSink = args.valueSink;
    }

    serialize(): string {
        assertNoCRLF(this.accessor);
        return [
            "R",
            this.parent,
            encodeBase64NoPadUtf8(this.accessor),
            this.valueSink,
        ].join("\n");
    }
}

export class SendMessage implements Message {
    readonly reference: UUID;
    readonly value: ParamValue;

    constructor(args: { reference: UUID; value: ParamValue }) {
        this.reference = args.reference;
        this.value = args.value;
    }

    serialize(): string {
        return ["S", this.reference, encodeParameterLine(this.value)].join(
            "\n",
        );
    }
}

export class ErrorMessage implements Message {
    readonly reference: UUID;
    readonly error: ParamValue;

    constructor(args: { reference: UUID; error: ParamValue }) {
        this.reference = args.reference;
        this.error = args.error;
    }

    serialize(): string {
        return ["E", this.reference, encodeParameterLine(this.error)].join(
            "\n",
        );
    }
}

export class Communication {
    private readonly socket: MessageSocket;

    private readonly callHandlers = new Set<
        (m: CallMessage) => void | Promise<void>
    >();
    private readonly requestHandlers = new Set<
        (m: RequestMessage) => void | Promise<void>
    >();
    private readonly sendHandlers = new Set<
        (m: SendMessage) => void | Promise<void>
    >();
    private readonly errorHandlers = new Set<
        (m: ErrorMessage) => void | Promise<void>
    >();

    private active = false;

    constructor(socket: MessageSocket) {
        this.socket = socket;
    }

    onCall(handler: (m: CallMessage) => void | Promise<void>): () => void {
        this.callHandlers.add(handler);
        return () => this.callHandlers.delete(handler);
    }

    onRequest(
        handler: (m: RequestMessage) => void | Promise<void>,
    ): () => void {
        this.requestHandlers.add(handler);
        return () => this.requestHandlers.delete(handler);
    }

    onSend(handler: (m: SendMessage) => void | Promise<void>): () => void {
        this.sendHandlers.add(handler);
        return () => this.sendHandlers.delete(handler);
    }

    onError(handler: (m: ErrorMessage) => void | Promise<void>): () => void {
        this.errorHandlers.add(handler);
        return () => this.errorHandlers.delete(handler);
    }

    async send(message: Message): Promise<void> {
        await this.socket.sendText(message.serialize());
    }

    /**
     * Starts accepting incoming messages from the socket and dispatching them
     * to subscribers. To stop accepting messages, invoke `close()`.
     */
    async run(): Promise<void> {
        if (this.active) {
            throw new Error("Communication.run() is already running");
        }
        this.active = true;
        try {
            while (true) {
                const text = await this.socket.receiveText();
                if (text === null) return;
                const parsed = parseWireMessage(text);
                await this.dispatch(parsed);
            }
        } finally {
            this.active = false;
        }
    }

    close(): void {
        this.socket.close();
    }

    /**
     * Delivers incoming parsed messages to subscribers.
     */
    private async dispatch(message: Message): Promise<void> {
        if (message instanceof CallMessage) {
            await invokeHandlers(this.callHandlers, message, "CallMessage");
            return;
        }
        if (message instanceof RequestMessage) {
            await invokeHandlers(
                this.requestHandlers,
                message,
                "RequestMessage",
            );
            return;
        }
        if (message instanceof SendMessage) {
            await invokeHandlers(this.sendHandlers, message, "SendMessage");
            return;
        }
        if (message instanceof ErrorMessage) {
            await invokeHandlers(this.errorHandlers, message, "ErrorMessage");
            return;
        }
        throw new Error("Unknown message instance");
    }
}

async function invokeHandlers<T>(
    handlers: Set<(m: T) => void | Promise<void>>,
    message: T,
    label: string,
): Promise<void> {
    for (const handler of handlers) {
        try {
            await handler(message);
        } catch (err) {
            // Handler errors are not fatal by design.
            console.error(`${label} handler error:`, err);
        }
    }
}

/* PARSING */

function parseWireMessage(text: string): Message {
    if (text.includes("\r")) {
        throw new Error("Invalid message: CR (\\r) is not allowed");
    }

    const lines = text.split("\n");
    if (lines.length === 0) {
        throw new Error("Invalid message: empty");
    }

    const kind = lines[0];
    switch (kind) {
        case "C":
            return parseCall(lines);
        case "R":
            return parseRequest(lines);
        case "S":
            return parseSend(lines);
        case "E":
            return parseError(lines);
        default:
            throw new Error(`Invalid message kind: ${kind}`);
    }
}

function parseCall(lines: string[]): CallMessage {
    if (lines.length < 3) {
        throw new Error("Invalid Call message: expected at least 3 lines");
    }
    const invocationPath = decodeBase64NoPadUtf8(lines[1]);
    const returnSink = lines[2];
    const positional: ParamValue[] = [];
    const named = new Map<string, ParamValue>();
    for (let i = 3; i < lines.length; i++) {
        const line = lines[i];
        if (line.length === 0) {
            throw new Error("Invalid Call message: empty parameter line");
        }
        const { value, name } = decodeParameterLine(line);
        if (name !== undefined) {
            // Technically, all named parameters have to have a unique name,
            // but here, we simply use the last parameter if there are duplicates.
            named.set(name, value);
        } else {
            positional.push(value);
        }
    }
    return new CallMessage({ invocationPath, returnSink, positional, named });
}

function parseRequest(lines: string[]): RequestMessage {
    if (lines.length !== 4) {
        throw new Error("Invalid Request message: expected exactly 4 lines");
    }
    const parent = lines[1];
    const accessor = decodeBase64NoPadUtf8(lines[2]);
    const valueSink = lines[3];
    return new RequestMessage({ parent, accessor, valueSink });
}

function parseSend(lines: string[]): SendMessage {
    // S\n<reference uuid>\n<param>
    if (lines.length !== 3) {
        throw new Error("Invalid Send message: expected exactly 3 lines");
    }
    const reference = lines[1];
    const { value, name } = decodeParameterLine(lines[2]);
    if (name !== undefined) {
        throw new Error("Invalid Send message: named parameter is not allowed");
    }
    return new SendMessage({ reference, value });
}

function parseError(lines: string[]): ErrorMessage {
    // E\n<reference uuid>\n<param>
    if (lines.length !== 3) {
        throw new Error("Invalid Error message: expected exactly 3 lines");
    }
    const reference = lines[1];
    const { value, name } = decodeParameterLine(lines[2]);
    if (name !== undefined) {
        throw new Error(
            "Invalid Error message: named parameter is not allowed",
        );
    }
    return new ErrorMessage({ reference, error: value });
}

function assertNoCRLF(text: string): void {
    if (text.includes("\r") || text.includes("\n")) {
        throw new Error("Invalid value: contains newline characters");
    }
}

/* PARAMETERS */

type WireParameter = {
    value: ParamValue;
    name?: string;
};

function decodeParameterLine(line: string): WireParameter {
    const space = line.indexOf(" ");
    const main = space === -1 ? line : line.slice(0, space);
    const nameB64 = space === -1 ? undefined : line.slice(space + 1);
    if (main.length < 2) {
        throw new Error("Invalid parameter: too short");
    }
    const descriptor = main[0];
    const raw = main.slice(1);
    let value: ParamValue;
    switch (descriptor) {
        case "i":
            value = { kind: "integer", value: parseSignedHex(raw) };
            break;
        case "f": {
            const n = Number(raw);
            if (!Number.isFinite(n)) {
                throw new Error(`Invalid float parameter: ${raw}`);
            }
            value = { kind: "float", value: n };
            break;
        }
        case "b":
            if (raw === "0") value = { kind: "boolean", value: false };
            else if (raw === "1") value = { kind: "boolean", value: true };
            else throw new Error(`Invalid boolean parameter: ${raw}`);
            break;
        case "s":
            value = { kind: "string", value: decodeBase64NoPadUtf8(raw) };
            break;
        case "r":
            value = { kind: "reference", value: raw };
            break;
        default:
            throw new Error(`Invalid parameter descriptor: ${descriptor}`);
    }
    const name = nameB64 === undefined
        ? undefined
        : decodeBase64NoPadUtf8(nameB64);
    if (name !== undefined) {
        assertNoCRLF(name);
    }
    return name === undefined ? { value } : { value, name };
}

function encodeParameterLine(value: ParamValue, name?: string): string {
    const main = encodeParameterValue(value);
    if (name === undefined) return main;
    assertNoCRLF(name);
    return main + " " + encodeBase64NoPadUtf8(name);
}

function encodeParameterValue(value: ParamValue): string {
    switch (value.kind) {
        case "integer":
            return "i" + formatSignedHex(value.value);
        case "float":
            return "f" + String(value.value);
        case "boolean":
            return "b" + (value.value ? "1" : "0");
        case "string":
            assertNoCRLF(value.value);
            return "s" + encodeBase64NoPadUtf8(value.value);
        case "reference":
            return "r" + value.value;
    }
}

function parseSignedHex(raw: string): number {
    // Accept: [-]?[0-9a-fA-F]+
    if (raw.length === 0) throw new Error("Invalid integer parameter: empty");
    const negative = raw[0] === "-";
    const digits = negative ? raw.slice(1) : raw;
    if (digits.length === 0) {
        throw new Error("Invalid integer parameter: missing digits");
    }
    for (let i = 0; i < digits.length; i++) {
        const c = digits.charCodeAt(i);
        const isHex = (c >= 48 && c <= 57) || (c >= 65 && c <= 70) ||
            (c >= 97 && c <= 102);
        if (!isHex) throw new Error(`Invalid integer parameter: ${raw}`);
    }
    const n = parseInt(digits, 16);
    if (!Number.isFinite(n)) {
        throw new Error(`Invalid integer parameter: ${raw}`);
    }
    return negative ? -n : n;
}

function formatSignedHex(n: number): string {
    if (!Number.isFinite(n)) {
        throw new Error(`Invalid integer value: ${n}`);
    }
    const i = n < 0 ? Math.ceil(n) : Math.floor(n);
    const abs = Math.abs(i);
    const hex = abs.toString(16);
    return i < 0 ? "-" + hex : hex;
}

/* TEXTUAL ENCODING/DECODING */

function encodeBase64NoPadUtf8(text: string): string {
    assertNoCRLF(text);
    const bytes = utf8Encode(text);
    const bufferCtor = getGlobalBufferCtor();
    if (bufferCtor) {
        return bufferCtor.from(bytes).toString("base64").replace(/=+$/g, "");
    }
    const btoaFn = getGlobalBtoa();
    if (!btoaFn) {
        throw new Error("Base64 encoding not supported in this runtime");
    }
    let binary = "";
    for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return btoaFn(binary).replace(/=+$/g, "");
}

function decodeBase64NoPadUtf8(b64: string): string {
    if (b64.includes("\r") || b64.includes("\n")) {
        throw new Error("Invalid base64: contains newline characters");
    }
    const padded = addBase64Padding(b64);
    const bufferCtor = getGlobalBufferCtor();
    if (bufferCtor) {
        const bytes = bufferCtor.from(padded, "base64");
        return utf8Decode(bytes);
    }
    const atobFn = getGlobalAtob();
    if (!atobFn) {
        throw new Error("Base64 decoding not supported in this runtime");
    }
    const binary = atobFn(padded);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
        bytes[i] = binary.charCodeAt(i);
    }
    return utf8Decode(bytes);
}

function addBase64Padding(b64: string): string {
    const rem = b64.length % 4;
    if (rem === 0) return b64;
    if (rem === 2) return b64 + "==";
    if (rem === 3) return b64 + "=";
    throw new Error("Invalid base64: incorrect length");
}

function utf8Encode(text: string): Uint8Array {
    if (typeof TextEncoder !== "undefined") {
        return new TextEncoder().encode(text);
    }
    const bufferCtor = getGlobalBufferCtor();
    if (bufferCtor) {
        return bufferCtor.from(text, "utf8");
    }
    throw new Error("UTF-8 encoding not supported in this runtime");
}

function utf8Decode(bytes: Uint8Array): string {
    if (typeof TextDecoder !== "undefined") {
        return new TextDecoder().decode(bytes);
    }
    const bufferCtor = getGlobalBufferCtor();
    if (bufferCtor) {
        return bufferCtor.from(bytes).toString("utf8");
    }
    throw new Error("UTF-8 decoding not supported in this runtime");
}

type BufferLike = {
    from(data: Uint8Array): { toString(enc: "base64" | "utf8"): string };
    from(data: string, enc: "base64" | "utf8"): Uint8Array;
};

function getGlobalBufferCtor(): BufferLike | undefined {
    return (globalThis as unknown as { Buffer?: BufferLike }).Buffer;
}

function getGlobalBtoa(): ((s: string) => string) | undefined {
    return (globalThis as unknown as { btoa?: (s: string) => string }).btoa;
}

function getGlobalAtob(): ((s: string) => string) | undefined {
    return (globalThis as unknown as { atob?: (s: string) => string }).atob;
}
