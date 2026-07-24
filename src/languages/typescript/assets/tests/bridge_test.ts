import assert from "node:assert/strict";
import test from "node:test";
import {
    AcknowledgeMessage,
    Bridge,
    CallMessage,
    DropMessage,
    ErrorMessage,
    MethodMessage,
    RequestMessage,
    SendMessage,
    UpdateMessage,
    type Message,
    type Parameter,
} from "../bridge.ts";
import { MessageSocket } from "../socket.ts";
import { MemoryDuplexStream } from "./stream.ts";

function roundTrip(message: Message): Message {
    const [a, b] = MemoryDuplexStream.pair();
    const sender = new Bridge(new MessageSocket(a));
    const receiver = new Bridge(new MessageSocket(b));

    sender.send(message);
    const parsed = receiver.nextMessage();
    assert.ok(parsed);
    assert.equal(parsed.kind, message.kind);
    return parsed;
}

test("Bridge: round-trips CallMessage with positional and named parameters", () => {
    const named = new Map<string, Parameter>([
        ["x", { kind: "integer", value: 255 }],
        ["title", { kind: "string", value: "Hello ü" }],
    ]);

    const parsed = roundTrip(
        new CallMessage({
            modulePath: "foo/bar",
            callee: { kind: "function", name: "sum" },
            returnSink: "return-uuid",
            positional: [
                { kind: "integer", value: -1 },
                { kind: "float", value: 3.5 },
                { kind: "boolean", value: true },
                { kind: "string", value: "hi" },
                { kind: "reference", value: "ref-uuid" },
            ],
            named,
        }),
    ) as CallMessage;

    assert.equal(parsed.modulePath, "foo/bar");
    assert.deepEqual(parsed.callee, { kind: "function", name: "sum" });
    assert.equal(parsed.returnSink, "return-uuid");
    assert.deepEqual(parsed.positionalParameters, [
        { kind: "integer", value: -1 },
        { kind: "float", value: 3.5 },
        { kind: "boolean", value: true },
        { kind: "string", value: "hi" },
        { kind: "reference", value: "ref-uuid" },
    ]);
    assert.deepEqual(parsed.namedParameters.get("x"), { kind: "integer", value: 255 });
    assert.deepEqual(parsed.namedParameters.get("title"), {
        kind: "string",
        value: "Hello ü",
    });
});

test("Bridge: round-trips static method calls", () => {
    const parsed = roundTrip(
        new CallMessage({
            modulePath: "pkg/util",
            callee: { kind: "staticMethod", typeName: "Widget", methodName: "create" },
            returnSink: "sink",
        }),
    ) as CallMessage;

    assert.deepEqual(parsed.callee, {
        kind: "staticMethod",
        typeName: "Widget",
        methodName: "create",
    });
});

test("Bridge: round-trips the remaining message kinds", () => {
    const messages: Message[] = [
        new MethodMessage({
            calledReference: "ref-1",
            methodName: "doThing",
            returnSink: "sink-1",
            positional: [{ kind: "integer", value: 42 }],
            named: new Map<string, Parameter>([["label", { kind: "string", value: "ready" }]]),
        }),
        new RequestMessage({
            parent: "parent-1",
            accessor: "field.name",
            valueSink: "value-1",
        }),
        new UpdateMessage({
            parent: "parent-1",
            accessor: "field.name",
            acknowledgeSink: "ack-1",
            value: { kind: "boolean", value: false },
        }),
        new SendMessage({
            reference: "ref-2",
            value: { kind: "string", value: "payload" },
        }),
        new AcknowledgeMessage({ reference: "ref-2" }),
        new ErrorMessage({
            reference: "ref-2",
            error: { kind: "string", value: "boom" },
        }),
        new DropMessage({ reference: "ref-2" }),
    ];

    const [a, b] = MemoryDuplexStream.pair();
    const sender = new Bridge(new MessageSocket(a));
    const receiver = new Bridge(new MessageSocket(b));

    for (const message of messages) {
        sender.send(message);
        const parsed = receiver.nextMessage();
        assert.ok(parsed);
        assert.equal(parsed.kind, message.kind);
        assert.deepEqual(parsed, message);
    }
});

test("Bridge: nextMessage returns null on EOF", () => {
    const [a, b] = MemoryDuplexStream.pair();
    a.close();
    const bridge = new Bridge(new MessageSocket(b));
    assert.equal(bridge.nextMessage(), null);
});

test("Bridge: invalid message kind throws", () => {
    const [a, b] = MemoryDuplexStream.pair();
    const socket = new MessageSocket(a);
    const bridge = new Bridge(new MessageSocket(b));

    socket.sendText("X");
    assert.throws(() => bridge.nextMessage(), /Invalid message kind/);
});

test("Bridge: serialize rejects CR/LF in fields", () => {
    assert.throws(
        () => {
            new CallMessage({
                modulePath: "bad\npath",
                callee: { kind: "function", name: "sum" },
                returnSink: "r",
            }).serialize();
        },
        /contains newline/,
    );
});
