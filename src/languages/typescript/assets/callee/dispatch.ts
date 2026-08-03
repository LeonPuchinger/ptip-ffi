/* {{IMPORTS}} */

import {
  AcknowledgeMessage,
  Bridge,
  CallMessage,
  ErrorMessage,
  Message,
  MethodMessage,
  Parameter,
  RequestMessage,
  SendMessage,
  UpdateMessage,
} from "./bridge.ts";

const instanceRegistry = new Map<string, unknown>();

type SerializedKind = "number" | "string" | "boolean" | "reference" | "json";

function decodeParameter(parameter: Parameter, kind: SerializedKind): unknown {
  switch (kind) {
    case "number": {
      if (parameter.kind === "integer" || parameter.kind === "float") {
        return parameter.value;
      }
      break;
    }
    case "string": {
      if (parameter.kind === "string") {
        return parameter.value;
      }
      break;
    }
    case "boolean": {
      if (parameter.kind === "boolean") {
        return parameter.value;
      }
      break;
    }
    case "reference": {
      if (parameter.kind === "reference") {
        const instance = instanceRegistry.get(parameter.value);
        if (instance === undefined) {
          throw new Error(`Instance ${parameter.value} not found`);
        }
        return instance;
      }
      break;
    }
    case "json": {
      if (parameter.kind === "string") {
        return JSON.parse(parameter.value);
      }
      break;
    }
  }

  throw new Error("Unexpected message kind");
}

function serializeValue(
  value: unknown,
  kind: SerializedKind,
  referenceSink: string,
): Parameter {
  switch (kind) {
    case "number": {
      if (typeof value !== "number") {
        throw new Error("Unexpected result type");
      }
      return {
        kind: Number.isInteger(value) ? "integer" : "float",
        value,
      };
    }
    case "string": {
      if (typeof value !== "string") {
        throw new Error("Unexpected result type");
      }
      return { kind: "string", value };
    }
    case "boolean": {
      if (typeof value !== "boolean") {
        throw new Error("Unexpected result type");
      }
      return { kind: "boolean", value };
    }
    case "reference": {
      if (value === null || typeof value !== "object") {
        throw new Error("Unexpected result type");
      }
      instanceRegistry.set(referenceSink, value);
      return { kind: "reference", value: referenceSink };
    }
    case "json": {
      return { kind: "string", value: JSON.stringify(value) };
    }
  }
}

export function dispatchMessage(message: Message, bridge: Bridge) {
  message.match({
    call(message) {
      const result = dispatchFunction(
        message.modulePath,
        message.callee,
        message.returnSink,
        message.positionalParameters,
        message.namedParameters,
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
      const result = dispatchMethod(
        instance,
        message.methodName,
        message.positionalParameters,
        message.namedParameters,
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
      handleUpdate(instance, message.accessor, message.value);
      bridge.send(new AcknowledgeMessage({ reference: message.acknowledgeSink }));
    },
    request(message) {
      const instance = instanceRegistry.get(message.parent);
      const result = handleRequest(instance, message.accessor, message.valueSink);
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
  positionalParameters: Parameter[],
  namedParameters: Map<string, Parameter> = new Map(),
): Parameter {
  switch (callee.kind) {
    case "function": {
      switch (modulePath) {
/* {{FUNCTION_CASES}} */
      }
      break;
    }
    case "staticMethod": {
      switch (modulePath) {
/* {{STATIC_METHOD_CASES}} */
      }
      break;
    }
  }

  throw new Error(`Function ${modulePath}.${"name" in callee ? callee.name : `${callee.typeName}.${callee.methodName}`} not found`);
}

function dispatchMethod(
  instance: unknown,
  methodName: string,
  positionalParameters: Parameter[],
  namedParameters: Map<string, Parameter> = new Map(),
  returnSink: string,
): Parameter {
/* {{METHOD_CASES}} */
  throw new Error(`Method ${methodName} not found`);
}

function handleRequest(
  parent: unknown,
  accessor: string,
  valueSink: string,
): Parameter {
  if (parent === null || typeof parent !== "object") {
    throw new Error(`Invalid parent for request ${accessor}`);
  }
/* {{REQUEST_CASES}} */
  throw new Error(`Property ${accessor} not found`);
}

function handleUpdate(
  parent: unknown,
  accessor: string,
  value: Parameter,
): void {
  if (parent === null || typeof parent !== "object") {
    throw new Error(`Invalid parent for update ${accessor}`);
  }
/* {{UPDATE_CASES}} */
  throw new Error(`Property ${accessor} not found`);
}