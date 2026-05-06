import {
    assertEquals,
    assertRejects,
    assertThrows,
} from "jsr:@std/assert@1.0.19";
import {
    CallMessage,
    Bridge,
    ErrorMessage,
    RequestMessage,
    SendMessage,
} from "../bridge.ts";
import { MessageSocket } from "../socket.ts";
import { MemoryDuplexStream } from "./stream.ts";

function deferred<T>() {
    let resolve!: (value: T) => void;
    let reject!: (reason?: unknown) => void;
    const promise = new Promise<T>((res, rej) => {
        resolve = res;
        reject = rej;
    });
    return { promise, resolve, reject };
}

Deno.test("Bridge: CallMessage roundtrip (positional + named)", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const commA = new Bridge(new MessageSocket(a));
    const commB = new Bridge(new MessageSocket(b));

    const got = deferred<CallMessage>();
    commB.onCall((m) => got.resolve(m));

    const runB = commB.run();

    const named = new Map();
    named.set("x", { kind: "integer", value: 255 });
    named.set("title", { kind: "string", value: "Hello ü" });

    await commA.send(
        new CallMessage({
            invocationPath: "foo.bar/baz",
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
    );

    const m = await got.promise;
    assertEquals(m.invocationPath, "foo.bar/baz");
    assertEquals(m.returnSink, "return-uuid");
    assertEquals(m.positionalParameters, [
        { kind: "integer", value: -1 },
        { kind: "float", value: 3.5 },
        { kind: "boolean", value: true },
        { kind: "string", value: "hi" },
        { kind: "reference", value: "ref-uuid" },
    ]);
    assertEquals(m.namedParameters.get("x"), { kind: "integer", value: 255 });
    assertEquals(m.namedParameters.get("title"), {
        kind: "string",
        value: "Hello ü",
    });

    // Shutdown the run loop cleanly.
    commA.close();
    await runB;
});

Deno.test("Bridge: Request/Send/Error message roundtrips", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const commA = new Bridge(new MessageSocket(a));
    const commB = new Bridge(new MessageSocket(b));

    const gotRequest = deferred<RequestMessage>();
    const gotSend = deferred<SendMessage>();
    const gotError = deferred<ErrorMessage>();
    commB.onRequest((m) => gotRequest.resolve(m));
    commB.onSend((m) => gotSend.resolve(m));
    commB.onError((m) => gotError.resolve(m));

    const runB = commB.run();

    await commA.send(
        new RequestMessage({
            parent: "p",
            accessor: "field.name",
            valueSink: "v",
        }),
    );
    await commA.send(
        new SendMessage({
            reference: "r",
            value: { kind: "boolean", value: false },
        }),
    );
    await commA.send(
        new ErrorMessage({
            reference: "r",
            error: { kind: "string", value: "boom" },
        }),
    );

    const r = await gotRequest.promise;
    assertEquals(r.parent, "p");
    assertEquals(r.accessor, "field.name");
    assertEquals(r.valueSink, "v");

    const s = await gotSend.promise;
    assertEquals(s.reference, "r");
    assertEquals(s.value, { kind: "boolean", value: false });

    const e = await gotError.promise;
    assertEquals(e.reference, "r");
    assertEquals(e.error, { kind: "string", value: "boom" });

    commA.close();
    await runB;
});

Deno.test("Bridge: handler errors are non-fatal", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const commA = new Bridge(new MessageSocket(a));
    const commB = new Bridge(new MessageSocket(b));

    const originalConsoleError = console.error;
    console.error = () => {};
    try {
        const called = deferred<void>();
        commB.onSend(() => {
            throw new Error("handler boom");
        });
        commB.onSend(() => {
            called.resolve();
        });

        const runB = commB.run();
        await commA.send(
            new SendMessage({
                reference: "r",
                value: { kind: "integer", value: 1 },
            }),
        );
        await called.promise;

        commA.close();
        await runB;
    } finally {
        console.error = originalConsoleError;
    }
});

Deno.test("Bridge: run() is not re-entrant", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const commB = new Bridge(new MessageSocket(b));
    const run1 = commB.run();

    await assertRejects(
        () => commB.run(),
        Error,
        "already running",
    );

    // End run1 by closing the remote side.
    a.close();
    await run1;
});

Deno.test("Bridge: invalid message kind rejects run() and allows restart", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const sockA = new MessageSocket(a);
    const commB = new Bridge(new MessageSocket(b));

    const run1 = commB.run();
    await sockA.sendText("X");
    await assertRejects(() => run1, Error, "Invalid message kind");

    // After failure, run() should be callable again.
    const run2 = commB.run();
    a.close();
    await run2;
});

Deno.test("Bridge: serialize() rejects CR/LF in fields", () => {
    assertThrows(
        () => {
            // invocationPath is checked for CR/LF.
            new CallMessage({
                invocationPath: "bad\npath",
                returnSink: "r",
            }).serialize();
        },
        Error,
        "contains newline",
    );
});
