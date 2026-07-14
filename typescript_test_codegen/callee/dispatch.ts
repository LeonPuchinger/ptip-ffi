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
      const positionalParameters = message.positionalParameters.map((param) => param.value);
      const namedParameters = new Map<string, any>();
      for (const [key, param] of message.namedParameters.entries()) {
        namedParameters.set(key, param.value);
      }
      const result = dispatchFunction(
        message.invocationPath,
        positionalParameters,
        namedParameters
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
