import { assertEquals, assertRejects } from "jsr:@std/assert@1.0.19";
import { MessageSocket } from "../socket.ts";
import { MemoryDuplexStream } from "./stream.ts";

Deno.test("MessageSocket: sendText/receiveText roundtrip", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const sender = new MessageSocket(a);
    const receiver = new MessageSocket(b);

    await sender.sendText("hello");
    assertEquals(await receiver.receiveText(), "hello");

    await sender.sendText("world");
    assertEquals(await receiver.receiveText(), "world");
});

Deno.test("MessageSocket: receives with heavy fragmentation", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    // Force tiny reads to ensure `receive()` loops.
    b.maxReadSize = 1;

    const sender = new MessageSocket(a);
    const receiver = new MessageSocket(b);

    await sender.sendText("fragmented message");
    assertEquals(await receiver.receiveText(), "fragmented message");
});

Deno.test("MessageSocket: parses multiple netstrings from one chunk", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);

    const enc = new TextEncoder();
    await a.write(enc.encode("5:hello,5:world,"));

    assertEquals(await receiver.receiveText(), "hello");
    assertEquals(await receiver.receiveText(), "world");
});

Deno.test("MessageSocket: EOF returns null", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);
    a.close();
    assertEquals(await receiver.receiveText(), null);
});

Deno.test("MessageSocket: invalid netstring (non-digit in length) throws", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);
    const enc = new TextEncoder();
    await a.write(enc.encode("x:abc,"));

    await assertRejects(
        () => receiver.receive(),
        Error,
        "Invalid netstring",
    );
});

Deno.test("MessageSocket: invalid netstring (missing comma) throws", async () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);
    const enc = new TextEncoder();
    // Looks like a netstring header but ends with '.' instead of ','.
    await a.write(enc.encode("3:abc."));

    await assertRejects(
        () => receiver.receive(),
        Error,
        "missing comma",
    );
});
