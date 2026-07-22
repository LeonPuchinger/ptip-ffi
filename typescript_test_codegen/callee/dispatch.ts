import {
  Bridge,
  Message,
  Parameter,
  SendMessage,
} from "./bridge.ts";
import { Point, takes_point, trim_whitespace } from "./library/index.ts";

const instanceRegistry = new Map<string, unknown>();

export function dispatchMessage(
  message: Message,
  bridge: Bridge,
) {
  message.match({
    call(message) {
      const positionalParameters = message.positionalParameters.map((param) => {
        if (param.kind === "reference") {
          const instance = instanceRegistry.get(param.value);
          if (instance === undefined) {
            throw new Error(`Instance ${param.value} not found`);
          }
          return instance;
        }
        return param.value;
      });
      const namedParameters = new Map<string, unknown>();
      for (const [key, param] of message.namedParameters.entries()) {
        let value: unknown;
        if (param.kind === "reference") {
          const instance = instanceRegistry.get(param.value);
          if (instance === undefined) {
            throw new Error(`Instance ${param.value} not found`);
          }
          value = instance;
        } else {
          value = param.value;
        }
        namedParameters.set(key, value);
      }
      const result = dispatchFunction(
        message.modulePath,
        message.callee,
        message.returnSink,
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
      const positionalParameters = message.positionalParameters.map((param) => {
        if (param.kind === "reference") {
          const instance = instanceRegistry.get(param.value);
          if (instance === undefined) {
            throw new Error(`Instance ${param.value} not found`);
          }
          return instance;
        }
        return param.value;
      });
      const namedParameters = new Map<string, unknown>();
      for (const [key, param] of message.namedParameters.entries()) {
        let value: unknown;
        if (param.kind === "reference") {
          const instance = instanceRegistry.get(param.value);
          if (instance === undefined) {
            throw new Error(`Instance ${param.value} not found`);
          }
          value = instance;
        } else {
          value = param.value;
        }
        namedParameters.set(key, value);
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
      console.warn("The callee cannot handle incoming send messages.")
    },
    error(_message) {
      console.warn("The callee cannot handle incoming error messages.")
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
        case "": {
          switch (callee.name) {
            case "Point": {
              const result = new Point(
                positionalParameters[0] as number,
                positionalParameters[1] as number,
              );
              instanceRegistry.set(returnSink, result);
              return { kind: "reference", value: returnSink };
            }
            case "trim_whitespace": {
              const result = trim_whitespace(positionalParameters[0] as string);
              return { kind: "string", value: result };
            }
            case "takes_point": {
              const result = takes_point(
                positionalParameters[0] as Parameters<typeof takes_point>[0],
              );
              return { kind: "float", value: result };
            }
          }
          break;
        }
      }
      throw new Error(`Function ${modulePath}.${callee.name} not found`);
    }
    case "staticMethod": {
      throw new Error(`Static method not found: ${modulePath}.${callee.typeName}.${callee.methodName}`);
    }
  }
}

function dispatchMethod(
  instance: unknown,
  methodName: string,
  positionalParameters: unknown[],
  namedParameters: Map<string, unknown> = new Map(),
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

function handleRequest(
  parent: unknown,
  accessor: string,
): Parameter {
  if (parent === null || typeof parent !== "object") {
    throw new Error(`Invalid parent for request ${accessor}`);
  }
  const value = (parent as Record<string, unknown>)[accessor];
  if (typeof value === "string") {
    return { kind: "string", value: value };
  }
  if (typeof value === "number") {
    return { kind: "float", value: value };
  }
  if (typeof value === "boolean") {
    return { kind: "boolean", value: value };
  }
  if (typeof value === "object" && value !== null) {
    for (const [ref, obj] of instanceRegistry.entries()) {
      if (obj === value) {
        return { kind: "reference", value: ref };
      }
    }
    const newReference = crypto.randomUUID();
    instanceRegistry.set(newReference, value);
    return { kind: "reference", value: newReference };
  }
  throw new Error(`Unsupported request value for ${accessor}`);
}
