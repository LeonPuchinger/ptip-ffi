    set /* {{NAME}} */(value: /* {{VALUE_TYPE}} */) {
        const acknowledgeSink = crypto.randomUUID();
        const bridge = establishBridge();
        bridge.send(
            new UpdateMessage({
                parent: this.uuid,
                accessor: "/* {{NAME}} */",
                acknowledgeSink: acknowledgeSink,
                value: /* {{VALUE}} */,
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