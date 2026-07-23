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

export function trim_whitespace(str: string): string {
    const returnSink = crypto.randomUUID();
    const bridge = establishBridge();
    bridge.send(
        new CallMessage({
            modulePath: "",
            callee: { "kind": "function", "name": "trim_whitespace" },
            returnSink: returnSink,
            positional: [{ kind: "string", value: str }],
            named: new Map(),
        }),
    );
    const response = bridge.nextMessage();
    if (response === null) {
        throw new Error("No response received from the bridge");
    }
    if (response.kind === "error") {
        throw (response as ErrorMessage).error.value;
    }
    if (response.kind === "send") {
        const sendMessage = response as SendMessage;
        if (sendMessage.reference !== returnSink) {
            throw new Error("Mismatched UUID in response");
        }
        if (sendMessage.value.kind !== "string") {
            throw new Error("Unexpected message kind");
        }
        return sendMessage.value.value;
    }
    throw new Error(`Unexpected message kind: ${response.kind}`);
}

export class Point {
    // TODO: think about how to make UUID references private to the library user,
    // but still accessible to the stubs...
    readonly uuid: string;

    constructor(x: number, y: number) {
        this.uuid = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new CallMessage({
                modulePath: "",
                callee: { kind: "function", name: "Point" },
                returnSink: this.uuid,
                positional: [{
                    kind: Number.isInteger(x) ? "integer" : "float",
                    value: x,
                }, {
                    kind: Number.isInteger(y) ? "integer" : "float",
                    value: y,
                }],
                named: new Map(),
            }),
        );
        const response = bridge.nextMessage();
        if (response === null) {
            throw new Error("No response received from the bridge");
        }
        if (response.kind === "error") {
            throw (response as ErrorMessage).error.value;
        }
        if (response.kind === "send") {
            const sendMessage = response as SendMessage;
            if (sendMessage.reference !== this.uuid) {
                throw new Error("Mismatched UUID in response");
            }
            if (
                sendMessage.value.kind !== "reference" ||
                sendMessage.value.value !== this.uuid
            ) {
                throw new Error("Unexpected message kind or value");
            }
        } else {
            throw new Error(`Unexpected message kind: ${response.kind}`);
        }
        finalizationRegistry.register(this, this.uuid);
    }

    get x(): number {
        const returnSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new RequestMessage({
                parent: this.uuid,
                accessor: "x",
                valueSink: returnSink,
            }),
        );
        const response = bridge.nextMessage();
        if (response === null) {
            throw new Error("No response received from the bridge");
        }
        if (response.kind === "error") {
            throw (response as ErrorMessage).error.value;
        }
        if (response.kind === "send") {
            const sendMessage = response as SendMessage;
            if (sendMessage.reference !== returnSink) {
                throw new Error("Mismatched UUID in response");
            }
            if (
                sendMessage.value.kind === "integer" ||
                sendMessage.value.kind === "float"
            ) {
                return sendMessage.value.value;
            } else {
                throw new Error("Unexpected message kind");
            }
        }
        throw new Error(`Unexpected message kind: ${response.kind}`);
    }

    set x(value: number) {
        const acknowledgeSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new UpdateMessage({
                parent: this.uuid,
                accessor: "x",
                acknowledgeSink: acknowledgeSink,
                value: {
                    kind: Number.isInteger(value) ? "integer" : "float",
                    value,
                },
            }),
        );
        const response = bridge.nextMessage();
        if (response === null) {
            throw new Error("No response received from the bridge");
        }
        if (response.kind === "error") {
            throw (response as ErrorMessage).error.value;
        }
        if (response.kind === "acknowledge") {
            const acknowledgeMessage = response as AcknowledgeMessage;
            if (acknowledgeMessage.reference !== acknowledgeSink) {
                throw new Error("Mismatched UUID in response");
            }
            return;
        }
        throw new Error(`Unexpected message kind: ${response.kind}`);
    }

    get y(): number {
        const returnSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new RequestMessage({
                parent: this.uuid,
                accessor: "y",
                valueSink: returnSink,
            }),
        );
        const response = bridge.nextMessage();
        if (response === null) {
            throw new Error("No response received from the bridge");
        }
        if (response.kind === "error") {
            throw (response as ErrorMessage).error.value;
        }
        if (response.kind === "send") {
            const sendMessage = response as SendMessage;
            if (sendMessage.reference !== returnSink) {
                throw new Error("Mismatched UUID in response");
            }
            if (
                sendMessage.value.kind === "integer" ||
                sendMessage.value.kind === "float"
            ) {
                return sendMessage.value.value;
            } else {
                throw new Error("Unexpected message kind");
            }
        }
        throw new Error(`Unexpected message kind: ${response.kind}`);
    }

    set y(value: number) {
        const acknowledgeSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new UpdateMessage({
                parent: this.uuid,
                accessor: "y",
                acknowledgeSink: acknowledgeSink,
                value: {
                    kind: Number.isInteger(value) ? "integer" : "float",
                    value,
                },
            }),
        );
        const response = bridge.nextMessage();
        if (response === null) {
            throw new Error("No response received from the bridge");
        }
        if (response.kind === "error") {
            throw (response as ErrorMessage).error.value;
        }
        if (response.kind === "acknowledge") {
            const acknowledgeMessage = response as AcknowledgeMessage;
            if (acknowledgeMessage.reference !== acknowledgeSink) {
                throw new Error("Mismatched UUID in response");
            }
            return;
        }
        throw new Error(`Unexpected message kind: ${response.kind}`);
    }

    distance_to_origin(): number {
        const returnSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new MethodMessage({
                calledReference: this.uuid,
                methodName: "distance_to_origin",
                returnSink: returnSink,
                positional: [],
                named: new Map(),
            }),
        );
        const response = bridge.nextMessage();
        if (response === null) {
            throw new Error("No response received from the bridge");
        }
        if (response.kind === "error") {
            throw (response as ErrorMessage).error.value;
        }
        if (response.kind === "send") {
            const sendMessage = response as SendMessage;
            if (sendMessage.reference !== returnSink) {
                throw new Error("Mismatched UUID in response");
            }
            if (
                sendMessage.value.kind === "integer" ||
                sendMessage.value.kind === "float"
            ) {
                return sendMessage.value.value;
            } else {
                throw new Error("Unexpected message kind");
            }
        }
        throw new Error(`Unexpected message kind: ${response.kind}`);
    }
}

export function takes_point(point: Point): number {
    const returnSink = crypto.randomUUID();
    const bridge = establishBridge();
    bridge.send(
        new CallMessage({
            modulePath: "",
            callee: { kind: "function", name: "takes_point" },
            returnSink: returnSink,
            positional: [{
                kind: "reference",
                value: point.uuid,
            }],
        }),
    );
    const response = bridge.nextMessage();
    if (response === null) {
        throw new Error("No response received from the bridge");
    }
    if (response.kind === "error") {
        throw (response as ErrorMessage).error.value;
    }
    if (response.kind === "send") {
        const sendMessage = response as SendMessage;
        if (sendMessage.reference !== returnSink) {
            throw new Error("Mismatched UUID in response");
        }
        if (
            sendMessage.value.kind === "integer" ||
            sendMessage.value.kind === "float"
        ) {
            return sendMessage.value.value;
        } else {
            throw new Error("Unexpected message kind");
        }
    }
    throw new Error(`Unexpected message kind: ${response.kind}`);
}
