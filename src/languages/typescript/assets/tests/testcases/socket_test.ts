import assert from "node:assert/strict";
import test from "node:test";
import { MessageSocket } from "../../socket.ts";
import { MemoryDuplexStream } from "./stream.ts";

test("MessageSocket: sendText/receiveText roundtrip", () => {
    const [a, b] = MemoryDuplexStream.pair();
    const sender = new MessageSocket(a);
    const receiver = new MessageSocket(b);

    sender.sendText("hello");
    assert.equal(receiver.receiveText(), "hello");

    sender.sendText("world");
    assert.equal(receiver.receiveText(), "world");
});

test("MessageSocket: receives with heavy fragmentation", () => {
    const [a, b] = MemoryDuplexStream.pair();
    b.maxReadSize = 1;

    const sender = new MessageSocket(a);
    const receiver = new MessageSocket(b);

    sender.sendText("fragmented message");
    assert.equal(receiver.receiveText(), "fragmented message");
});

test("MessageSocket: parses multiple netstrings from one chunk", () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);

    const enc = new TextEncoder();
    a.write(enc.encode("5:hello,5:world,"));

    assert.equal(receiver.receiveText(), "hello");
    assert.equal(receiver.receiveText(), "world");
});

test("MessageSocket: EOF returns null", () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);
    a.close();
    assert.equal(receiver.receiveText(), null);
});

test("MessageSocket: invalid netstring (non-digit in length) throws", () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);
    const enc = new TextEncoder();
    a.write(enc.encode("x:abc,"));

    assert.throws(() => receiver.receive(), /Invalid netstring/);
});

test("MessageSocket: invalid netstring (missing comma) throws", () => {
    const [a, b] = MemoryDuplexStream.pair();
    const receiver = new MessageSocket(b);
    const enc = new TextEncoder();
    a.write(enc.encode("3:abc."));

    assert.throws(() => receiver.receive(), /missing comma/);
});