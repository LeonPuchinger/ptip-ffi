import { Bridge } from "./bridge.ts";
import { MessageSocket, SynchronousSocketServer } from "./socket.ts";

const path = "/tmp/test_ptip_ffi.sock";

const advertiseSocket = (() => {
    let alreadyAdvertised = false;
    return () => {
        if (alreadyAdvertised) return;
        console.log(path);
        alreadyAdvertised = true;
    };
})();

const socketServer = new SynchronousSocketServer(path);
while (true) {
    const streamSocket = socketServer.accept();
    const datagramSocket = new MessageSocket(streamSocket);
    const bridge = new Bridge(datagramSocket);
    advertiseSocket();
    bridge.run();
    // TODO: hook up dispatch
}
