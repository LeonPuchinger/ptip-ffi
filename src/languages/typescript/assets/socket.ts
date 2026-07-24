import { createRequire } from "node:module";

type Bytes = Uint8Array<ArrayBufferLike>;

type NativeSynchronousSocket = {
  readIntoBuffer(buffer: Bytes): number | null;
  writeFromBuffer(buffer: Bytes): number;
  connect(): void;
  disconnect(): void;
};

type NativeSynchronousSocketServer = {
  listen(): void;
  accept(): NativeSynchronousSocket;
  close(): void;
};

type NativeSocketModule = {
  SynchronousSocket: new (path: string) => NativeSynchronousSocket;
  SynchronousSocketServer: new (path: string) => NativeSynchronousSocketServer;
};

const require = createRequire(import.meta.url);

function loadNativeSocketModule(): NativeSocketModule {
  return require("synchronous-socket") as NativeSocketModule;
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
  private socket: NativeSynchronousSocket

  constructor(socket: NativeSynchronousSocket) {
    this.socket = socket;
  }

  static fromPath(path: string) {
    const { SynchronousSocket } = loadNativeSocketModule();
    const socket = new SynchronousSocket(path);
    socket.connect();
    return new SynchronousSocket(socket);
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
    this.socket.disconnect();
  }
}

export class SynchronousSocketServer {
  private server: NativeSynchronousSocketServer;

  constructor(path: string) {
    const { SynchronousSocketServer } = loadNativeSocketModule();
    this.server = new SynchronousSocketServer(path);
    this.server.listen();
  }

  accept(): SynchronousSocket {
    const socket = this.server.accept();
    return new SynchronousSocket(socket);
  }

  close(): void {
    this.server.close();
  }
}

/**
 * A socket that can send and receive length-prefixed messages (netstrings) over a `Stream`,
 * such as a unix domain socket, implemented by `DenoConnection` or `NodeStream` for example.
 */
export class MessageSocket {
  private buffer: Bytes = new Uint8Array(0);
  private decoder = new TextDecoder();
  private encoder = new TextEncoder();
  private stream: SynchronousStream;

  constructor(conn: SynchronousStream) {
    this.stream = conn;
  }

  send(data: Bytes) {
    const header = this.encoder.encode(String(data.length) + ":");
    const trailer = this.encoder.encode(",");
    this.stream.write(header);
    this.stream.write(data);
    this.stream.write(trailer);
  }

  sendText(text: string) {
    this.send(this.encoder.encode(text));
  }

  receive(): Bytes | null {
    while (true) {
      const msg = this.tryParse();
      if (msg) return msg;
      const chunk = new Uint8Array(1024);
      const n = this.stream.read(chunk);
      if (n === null) return null;
      this.buffer = concat(this.buffer, chunk.subarray(0, n));
    }
  }

  receiveText(): string | null {
    const msg = this.receive();
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
