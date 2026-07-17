import {
  Bridge,
  Message,
  Parameter,
  SendMessage,
} from "./bridge.ts";
import { trim_whitespace } from "./library/index.ts";

const instanceRegistry = new Map<string, unknown>();

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
        message.modulePath,
        message.callee,
        positionalParameters,
        namedParameters
      );
      const response = new SendMessage({
        reference: message.returnSink,
        value: result,
      });
      bridge.send(response);
    },
    method(message) {
      const instance = instanceRegistry.get(message.calledReference);
      if (instance === undefined) {
        throw new Error(`Instance ${message.calledReference} not found`);
      }
      const positionalParameters = message.positionalParameters.map((param) => param.value);
      const namedParameters = new Map<string, any>();
      for (const [key, param] of message.namedParameters.entries()) {
        namedParameters.set(key, param.value);
      }
      const result = dispatchMethod(
        instance,
        message.methodName,
        positionalParameters,
        namedParameters,
      );
      const response = new SendMessage({
        reference: message.returnSink,
        value: result,
      });
      bridge.send(response);
    },
  });
}

export function dispatchFunction(
  modulePath: string[],
  callee: { kind: "function"; name: string } | { kind: "staticMethod"; typeName: string; methodName: string },
  positionalParameters: unknown[],
  namedParameters: Map<string, any> = new Map(),
): Parameter {
  const concatenatedModulePath = modulePath.join("/");
  switch (callee.kind) {
    case "function": {
      switch (concatenatedModulePath) {
        case "": {
          switch (callee.name) {
            case "trim_whitespace": {
              const result = trim_whitespace(positionalParameters[0] as string);
              return { kind: "string", value: result };
            }
          }
          break;
        }
      }
      throw new Error(`Function ${concatenatedModulePath}.${callee.name} not found`);
    }
    case "staticMethod": {
      throw new Error(`Static method not found: ${concatenatedModulePath}.${callee.typeName}.${callee.methodName}`);
    }
  }
}

function dispatchMethod(
  instance: unknown,
  methodName: string,
  positionalParameters: unknown[],
  _namedParameters: Map<string, any> = new Map(),
): Parameter {
  if (instance === null || typeof instance !== "object") {
    throw new Error(`Invalid instance for method ${methodName}`);
  }
  const method = (instance as Record<string, unknown>)[methodName];
  if (typeof method !== "function") {
    throw new Error(`Method ${methodName} not found`);
  }
  const result = (method as (...args: unknown[]) => unknown).apply(
    instance,
    positionalParameters,
  );
  if (typeof result === "string") {
    return { kind: "string", value: result };
  }
  if (typeof result === "number") {
    if (Number.isInteger(result)) {
      return { kind: "integer", value: result };
    }
    return { kind: "float", value: result };
  }
  if (typeof result === "boolean") {
    return { kind: "boolean", value: result };
  }
  if (typeof result === "object" && result !== null) {
    for (const [ref, obj] of instanceRegistry.entries()) {
      if (obj === result) {
        return { kind: "reference", value: ref };
      }
    }
    const newReference = crypto.randomUUID();
    instanceRegistry.set(newReference, result);
    return { kind: "reference", value: newReference };
  }
  throw new Error(`Unsupported method return value for ${methodName}`);
}
