import type { Stream } from "../socket.ts";

type Bytes = Uint8Array<ArrayBufferLike>;

type PendingRead = {
    buffer: Bytes;
    resolve: (n: number | null) => void;
    reject: (err: unknown) => void;
};

/**
 * In-memory, deterministic duplex stream for tests.
 *
 * - `write()` pushes bytes into the peer's read queue.
 * - `read()` resolves when data becomes available or on EOF.
 * - `close()` signals EOF to the peer.
 */
export class MemoryDuplexStream implements Stream {
    private peer: MemoryDuplexStream | null = null;
    private readonly queue: Bytes[] = [];
    private pending: PendingRead | null = null;
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

    async read(buffer: Bytes): Promise<number | null> {
        if (this.pending) {
            throw new Error("Concurrent read() calls are not supported");
        }

        const n = this.tryReadInto(buffer);
        if (n !== null) return n;

        if (this.remoteClosed) return null;

        return await new Promise<number | null>((resolve, reject) => {
            this.pending = { buffer, resolve, reject };
        });
    }

    write(buffer: Bytes): Promise<number> {
        if (this.closed) throw new Error("Stream is closed");
        if (!this.peer) throw new Error("Unpaired MemoryDuplexStream");
        this.peer.enqueue(buffer);
        return Promise.resolve(buffer.length);
    }

    close(): void {
        if (this.closed) return;
        this.closed = true;
        // Signal EOF to the peer.
        if (this.peer) this.peer.notifyRemoteClosed();
    }

    private enqueue(buffer: Bytes): void {
        // Copy to avoid shared backing buffers between tests.
        this.queue.push(buffer.slice());
        this.flushPending();
    }

    private notifyRemoteClosed(): void {
        this.remoteClosed = true;
        this.flushPending();
    }

    private flushPending(): void {
        if (!this.pending) return;
        const { buffer, resolve } = this.pending;
        const n = this.tryReadInto(buffer);
        if (n !== null) {
            this.pending = null;
            resolve(n);
            return;
        }
        if (this.remoteClosed) {
            this.pending = null;
            resolve(null);
        }
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
