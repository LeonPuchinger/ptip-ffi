        const returnSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new CallMessage({
                modulePath: "/* {{MODULE_PATH}} */",
                callee: { kind: "function", name: "/* {{CALLEE_NAME}} */" },
                returnSink: returnSink,
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
            if (sendMessage.reference !== returnSink) {
                throw new Error("Mismatched UUID in response");
            }
/* {{RESPONSE_BODY}} */
        }
        throw new Error(`Unexpected message kind: ${response.kind}`);