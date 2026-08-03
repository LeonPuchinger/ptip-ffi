    get /* {{NAME}} */(): /* {{RETURN_TYPE}} */ {
        const returnSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new RequestMessage({
                parent: this.uuid,
                accessor: "/* {{NAME}} */",
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
/* {{BODY}} */
        throw new Error(`Unexpected message kind: ${response.kind}`);
    }