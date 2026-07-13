import { Bridge, Message, Parameter, SendMessage } from "./bridge.ts";
import { trim_whitespace } from "./library/index.ts";

// deno-lint-ignore no-explicit-any
const instanceRegistry = new WeakMap<string, any>();

export function dispatchMessage(
  message: Message,
  bridge: Bridge,
) {
  message.match({
    call(message) {
      const result = dispatchFunction(
        message.invocationPath,
        message.positionalParameters,
        message.namedParameters
      );
      const response = new SendMessage({
        reference: message.returnSink,
        value: result,
      });
      bridge.send(response);
    }
  });
}

export function dispatchFunction(
  invocationPath: string,
  positionalParameters: any[],
  namedParameters: Map<string, any> = new Map(),
): Parameter {
  switch (invocationPath) {
    case "trim_whitespace": {
      const result = trim_whitespace(positionalParameters[0]);
      return { kind: "string", value: result };
    }
    default:
      throw new Error(`Function ${invocationPath} not found`);
  }
}
