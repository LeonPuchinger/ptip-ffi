export class SynchronousSocket {
  constructor(path) {
    this.path = path;
    this.connected = false;
  }

  connect() {
    this.connected = true;
  }

  readIntoBuffer(_buffer) {
    return null;
  }

  writeFromBuffer(buffer) {
    return buffer.length;
  }

  disconnect() {
    this.connected = false;
  }
}

export class SynchronousSocketServer {
  constructor(path) {
    this.path = path;
    this.listening = false;
  }

  listen() {
    this.listening = true;
  }

  accept() {
    return new SynchronousSocket(this.path);
  }

  close() {
    this.listening = false;
  }
}
