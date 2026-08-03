    constructor(/* {{PARAMETERS}} */) {
        this.uuid = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new CallMessage({
                modulePath: "/* {{MODULE_PATH}} */",
                callee: { kind: "function", name: "/* {{TYPE_NAME}} */" },
                returnSink: this.uuid,
                positional: [/* {{POSITIONAL_VALUES}} */],
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
            if (sendMessage.value.kind !== "reference" || sendMessage.value.value !== this.uuid) {
                throw new Error("Unexpected message kind or value");
            }
        } else {
            throw new Error(`Unexpected message kind: ${response.kind}`);
        }
        finalizationRegistry.register(this, this.uuid);
    }