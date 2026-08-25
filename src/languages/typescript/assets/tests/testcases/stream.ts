import type { SynchronousStream } from "../../socket.ts";

type Bytes = Uint8Array<ArrayBufferLike>;

/**
 * In-memory, deterministic duplex stream for tests.
 *
 * - `write()` pushes bytes into the peer's read queue.
 * - `read()` returns queued bytes immediately or throws if the test would block.
 * - `close()` signals EOF to the peer.
 */
export class MemoryDuplexStream implements SynchronousStream {
    private peer: MemoryDuplexStream | null = null;
    private readonly queue: Bytes[] = [];
    private closed = false;
    private remoteClosed = false;

    /**
     * Maximum number of bytes a single `read()` call may return.
     * Useful to simulate network fragmentation.
     */
    maxReadSize: number | null = null;

    static pair(): [MemoryDuplexStream, MemoryDuplexStream] {
        const a = new MemoryDuplexStream();
        const b = new MemoryDuplexStream();
        a.peer = b;
        b.peer = a;
        return [a, b];
    }

    read(buffer: Bytes): number | null {
        const n = this.tryReadInto(buffer);
        if (n !== null) return n;
        if (this.remoteClosed) return null;
        throw new Error("Read would block; write data before reading in tests");
    }

    write(buffer: Bytes): number {
        if (this.closed) throw new Error("Stream is closed");
        if (!this.peer) throw new Error("Unpaired MemoryDuplexStream");
        this.peer.enqueue(buffer);
        return buffer.length;
    }

    close(): void {
        if (this.closed) return;
        this.closed = true;
        if (this.peer) this.peer.notifyRemoteClosed();
    }

    private enqueue(buffer: Bytes): void {
        this.queue.push(buffer.slice());
    }

    private notifyRemoteClosed(): void {
        this.remoteClosed = true;
    }

    private tryReadInto(buffer: Bytes): number | null {
        if (this.queue.length === 0) return null;
        const head = this.queue[0];
        const limit = this.maxReadSize === null
            ? buffer.length
            : Math.min(buffer.length, this.maxReadSize);
        const n = Math.min(limit, head.length);
        buffer.set(head.subarray(0, n));
        if (n === head.length) {
            this.queue.shift();
        } else {
            this.queue[0] = head.subarray(n).slice();
        }
        return n;
    }
}