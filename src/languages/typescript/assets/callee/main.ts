import { Bridge } from "./bridge.ts";
import { dispatchMessage } from "./dispatch.ts";
import { MessageSocket, SynchronousSocketServer } from "./socket.ts";

const path = "/* {{SOCKET_PATH}} */";

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
    advertiseSocket();
    const streamSocket = socketServer.accept();
    const datagramSocket = new MessageSocket(streamSocket);
    const bridge = new Bridge(datagramSocket);
    while (true) {
        const message = bridge.nextMessage();
        if (message === null) {
            break;
        }
        dispatchMessage(message, bridge);
    }
}