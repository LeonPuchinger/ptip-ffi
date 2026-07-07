import { default as NativeSynchronousSocket } from "synchronous-socket";
import { runtimeEnvironment } from "./util.ts";

type Bytes = Uint8Array<ArrayBufferLike>;

/**
 * An abstraction over a byte stream that can be used for communication over a socket or similar transport.
 * The `read` method reads data into the provided buffer, returning the number of bytes read, or `null` on EOF.
 * The `write` method writes data from the provided buffer, returning the number of bytes written.
 * The `close` method closes the stream and releases any resources associated with it.
 */
export interface Stream {
  read(buffer: Bytes): Promise<number | null>;
  write(buffer: Bytes): Promise<number>;
  close(): void;
}

/**
 * An abstraction over a synchronous byte stream that can be used for communication over a socket or similar transport.
 * The `read` method reads data into the provided buffer, returning the number of bytes read, or `null` on EOF.
 * The `write` method writes data from the provided buffer, returning the number of bytes written.
 * The `close` method closes the stream and releases any resources associated with it.
 */
export interface SynchronousStream {
  read(buffer: Bytes): number | null;
  write(buffer: Bytes): number;
  close(): void;
}

/**
 * A wrapper around `synchronous-socket`, making it conform to the `SynchronousStream` interface.
 */
export class SynchronousSocket implements SynchronousStream {
  private socket: typeof NativeSynchronousSocket

  constructor(path: string) {
    this.socket = new NativeSynchronousSocket(path);
    this.socket.connect();
  }

  read(buffer: Bytes): number | null {
    const bytesRead = this.socket.readIntoBuffer(buffer);
    if (bytesRead === null || bytesRead === 0) {
      return null;
    }
    return bytesRead;
  }

  write(buffer: Bytes): number {
    return this.socket.writeFromBuffer(buffer);
  }

  close(): void {
    this.socket.close();
  }
}

/**
 * A wrapper around `Deno.Conn` for use with Deno.
 */
class DenoConnection implements Stream {
  private connection: Deno.Conn;

  constructor(connection: Deno.Conn) {
    this.connection = connection;
  }

  read(buffer: Bytes): Promise<number | null> {
    return this.connection.read(buffer);
  }

  write(buffer: Bytes): Promise<number> {
    return this.connection.write(buffer);
  }

  close() {
    this.connection.close();
  }
}

import { Buffer } from "node:buffer";
import net from "node:net";

/**
 * A wrapper around `net.Socket` for use with Node.js.
 */
export class NodeStream implements Stream {
  private readonly socket: net.Socket;
  private ended = false;
  private pendingRead: Promise<number | null> | null = null;

  constructor(socket: net.Socket) {
    this.socket = socket;
    // Start paused: only read when someone calls `read()`.
    this.socket.pause();

    const markEnded = () => {
      this.ended = true;
    };

    // Track EOF even if no read is currently pending.
    this.socket.on("end", markEnded);
    this.socket.on("close", markEnded);
  }

  read(buffer: Bytes): Promise<number | null> {
    // Optional safety: disallow concurrent reads (the rest of the code assumes 0 or 1 pending read).
    if (this.pendingRead) {
      throw new Error("Concurrent read() calls are not supported");
    }
    if (this.ended) return Promise.resolve(null);
    this.pendingRead = new Promise<number | null>((resolve) => {
      let settled = false;
      const cleanup = () => {
        if (settled) return;
        settled = true;
        this.socket.removeListener("data", onData);
        this.socket.removeListener("end", onEnd);
        this.socket.removeListener("close", onClose);
        // Backpressure lives in the socket: pause whenever no read is actively waiting.
        this.socket.pause();
        this.pendingRead = null;
      };

      const onData = (chunk: Buffer) => {
        cleanup();
        const n = Math.min(buffer.length, chunk.length);
        buffer.set(chunk.subarray(0, n));
        // Preserve remaining bytes for the next read.
        if (n < chunk.length) {
          const head = chunk.subarray(0, n);
          buffer.set(new Uint8Array(head));
          if (n < chunk.length) {
            const rest = chunk.subarray(n);
            this.socket.unshift(new Uint8Array(rest));
          }
        }
        resolve(n);
      };

      const onEnd = () => {
        this.ended = true;
        cleanup();
        resolve(null);
      };

      const onClose = () => {
        this.ended = true;
        cleanup();
        resolve(null);
      };

      this.socket.once("data", onData);
      this.socket.once("end", onEnd);
      this.socket.once("close", onClose);
      // Allow the socket to emit exactly one chunk (or EOF) for this read.
      this.socket.resume();
    });
    return this.pendingRead;
  }

  write(buffer: Bytes): Promise<number> {
    return new Promise((resolve, reject) => {
      this.socket.write(buffer, (err) => {
        if (err) reject(err);
        else resolve(buffer.length);
      });
    });
  }

  close(): void {
    this.socket.destroy();
  }
}

/**
 * Connects to a unix domain socket at the given path, returning a `Stream` for communication.
 * The caller is responsible for cleaning up the socket file when done.
 * Note: this function is not designed to be used concurrently from multiple processes, and does not
 * implement any locking around the socket file.
 * TLDR: Intended for use in a client process that connects to a server.
 */
export async function connectUnix(path: string): Promise<Stream> {
  const env = runtimeEnvironment();
  if (env === "deno") {
    const conn = await Deno.connect({ transport: "unix", path });
    return new DenoConnection(conn);
  }
  if (env === "node") {
    const net = await import("node:net");
    return new Promise((resolve, reject) => {
      const socket = net.createConnection(path, () => {
        resolve(new NodeStream(socket));
      });
      socket.on("error", reject);
    });
  }
  throw new Error("Runtime environment not supported");
}

/**
 * Listens for incoming connections on a unix domain socket at the given path, yielding a `Stream`
 * for each connection. The socket is created if it doesn't exist, and removed if it already exists.
 * The caller is responsible for cleaning up the socket file when done.
 * Note: this function is not designed to be used concurrently from multiple processes, and does not
 * implement any locking around the socket file.
 * TLDR: Intended for use in a server process that accepts connections from clients.
 */
export async function* listenUnix(path: string): AsyncIterable<Stream> {
  const environment = runtimeEnvironment();
  if (environment === "deno") {
    try {
      await Deno.remove(path);
    } catch {}
    const listener = Deno.listen({ transport: "unix", path });
    for await (const conn of listener) {
      yield new DenoConnection(conn);
    }
    return;
  }
  if (environment === "node") {
    const net = await import("node:net");
    const fs = await import("node:fs");
    try {
      fs.unlinkSync(path);
    } catch {}
    const server = net.createServer();
    server.listen(path);
    const queue: Stream[] = [];
    let resolve: ((c: Stream) => void) | null = null;
    server.on("connection", (socket) => {
      const connection = new NodeStream(socket);
      if (resolve) {
        resolve(connection);
        resolve = null;
      } else {
        queue.push(connection);
      }
    });
    while (true) {
      if (queue.length > 0) {
        yield queue.shift()!;
      } else {
        const conn = await new Promise<Stream>((res) => {
          resolve = res;
        });
        yield conn;
      }
    }
  }
  throw new Error("Runtime environment not supported");
}

/**
 * A socket that can send and receive length-prefixed messages (netstrings) over a `Stream`,
 * such as a unix domain socket, implemented by `DenoConnection` or `NodeStream` for example.
 */
export class MessageSocket {
  private buffer: Bytes = new Uint8Array(0);
  private decoder = new TextDecoder();
  private encoder = new TextEncoder();
  private stream: Stream;

  constructor(conn: Stream) {
    this.stream = conn;
  }

  async send(data: Bytes) {
    const header = this.encoder.encode(String(data.length) + ":");
    const trailer = this.encoder.encode(",");
    await this.stream.write(header);
    await this.stream.write(data);
    await this.stream.write(trailer);
  }

  async sendText(text: string) {
    await this.send(this.encoder.encode(text));
  }

  async receive(): Promise<Bytes | null> {
    while (true) {
      const msg = this.tryParse();
      if (msg) return msg;
      const chunk = new Uint8Array(1024);
      const n = await this.stream.read(chunk);
      if (n === null) return null;
      this.buffer = concat(this.buffer, chunk.subarray(0, n));
    }
  }

  async receiveText(): Promise<string | null> {
    const msg = await this.receive();
    return msg ? this.decoder.decode(msg) : null;
  }

  private tryParse(): Bytes | null {
    let colon = -1;
    for (let i = 0; i < this.buffer.length; i++) {
      const c = this.buffer[i];
      if (c === 58) {
        colon = i;
        break;
      }
      if (c < 48 || c > 57) throw new Error("Invalid netstring");
    }
    if (colon === -1) return null;
    const len = Number(this.decoder.decode(this.buffer.slice(0, colon)));
    const total = colon + 1 + len + 1;
    if (this.buffer.length < total) return null;
    const dataStart = colon + 1;
    const dataEnd = dataStart + len;
    if (this.buffer[dataEnd] !== 44) {
      throw new Error("Invalid netstring (missing comma)");
    }
    const msg = this.buffer.slice(dataStart, dataEnd);
    this.buffer = this.buffer.slice(total);
    return msg;
  }

  close() {
    this.stream.close();
  }
}

function concat(a: Bytes, b: Bytes): Bytes {
  const out = new Uint8Array(a.length + b.length);
  out.set(a);
  out.set(b, a.length);
  return out;
}
