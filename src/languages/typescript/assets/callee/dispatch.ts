/* {{IMPORTS}} */

import {
  AcknowledgeMessage,
  Bridge,
  Message,
  Parameter,
  SendMessage,
} from "./bridge.ts";

const instanceRegistry = new Map<string, unknown>();

function resolveParameterValue(param: Parameter): unknown {
  if (param.kind === "reference") {
    const instance = instanceRegistry.get(param.value);
    if (instance === undefined) {
      throw new Error(`Instance ${param.value} not found`);
    }
    return instance;
  }
  return param.value;
}

export function dispatchMessage(message: Message, bridge: Bridge) {
  message.match({
    call(message) {
      const positionalParameters = message.positionalParameters.map(resolveParameterValue);
      const namedParameters = new Map<string, unknown>();
      for (const [key, param] of message.namedParameters.entries()) {
        namedParameters.set(key, resolveParameterValue(param));
      }
      const result = dispatchFunction(
        message.modulePath,
        message.callee,
        message.returnSink,
        positionalParameters,
        namedParameters,
      );
      bridge.send(
        new SendMessage({
          reference: message.returnSink,
          value: result,
        }),
      );
    },
    method(message) {
      const instance = instanceRegistry.get(message.calledReference);
      if (instance === undefined) {
        throw new Error(`Instance ${message.calledReference} not found`);
      }
      const positionalParameters = message.positionalParameters.map(resolveParameterValue);
      const namedParameters = new Map<string, unknown>();
      for (const [key, param] of message.namedParameters.entries()) {
        namedParameters.set(key, resolveParameterValue(param));
      }
      const result = dispatchMethod(
        instance,
        message.methodName,
        positionalParameters,
        namedParameters,
        message.returnSink,
      );
      bridge.send(
        new SendMessage({
          reference: message.returnSink,
          value: result,
        }),
      );
    },
    update(message) {
      const instance = instanceRegistry.get(message.parent);
      if (instance === undefined) {
        throw new Error(`Instance ${message.parent} not found`);
      }
      if (instance === null || typeof instance !== "object") {
        throw new Error(`Invalid instance for update ${message.accessor}`);
      }
      (instance as Record<string, unknown>)[message.accessor] = resolveParameterValue(message.value);
      bridge.send(new AcknowledgeMessage({ reference: message.acknowledgeSink }));
    },
    request(message) {
      const instance = instanceRegistry.get(message.parent);
      const result = handleRequest(instance, message.accessor);
      const response = new SendMessage({
        reference: message.valueSink,
        value: result,
      });
      bridge.send(response);
    },
    send(_message) {
      console.warn("The callee cannot handle incoming send messages.");
    },
    error(_message) {
      console.warn("The callee cannot handle incoming error messages.");
    },
    drop(message) {
      instanceRegistry.delete(message.reference);
    },
  });
}

export function dispatchFunction(
  modulePath: string,
  callee: { kind: "function"; name: string } | { kind: "staticMethod"; typeName: string; methodName: string },
  returnSink: string,
  positionalParameters: unknown[],
  namedParameters: Map<string, unknown> = new Map(),
): Parameter {
  switch (callee.kind) {
    case "function": {
      switch (modulePath) {
/* {{FUNCTION_CASES}} */
      }
      break;
    }
    case "staticMethod": {
      throw new Error(`Static method not found: ${modulePath}.${callee.typeName}.${callee.methodName}`);
    }
  }

  throw new Error(`Function ${modulePath}.${"name" in callee ? callee.name : `${callee.typeName}.${callee.methodName}`} not found`);
}

function dispatchMethod(
  instance: unknown,
  methodName: string,
  positionalParameters: unknown[],
  namedParameters: Map<string, unknown> = new Map(),
  returnSink: string,
): Parameter {
/* {{METHOD_CASES}} */
  throw new Error(`Method ${methodName} not found`);
}

function handleRequest(
  parent: unknown,
  accessor: string,
): Parameter {
  if (parent === null || typeof parent !== "object") {
    throw new Error(`Invalid parent for request ${accessor}`);
  }
/* {{REQUEST_CASES}} */
  throw new Error(`Property ${accessor} not found`);
}
