import { AcknowledgeMessage, CallMessage, DropMessage, ErrorMessage, MethodMessage, RequestMessage, SendMessage, UpdateMessage } from "./ffi/bridge.ts";
import { establishBridge } from "./ffi/main.ts";

const finalizationRegistry = new FinalizationRegistry((uuid: string) => {
    const bridge = establishBridge();
    bridge.send(
        new DropMessage({
            reference: uuid,
        })
    );
});

/* {{STUBS}} */
