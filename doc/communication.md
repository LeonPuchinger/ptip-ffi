# Communication

Both sides of the FFI need to communicate over a shared channel to exchange data and instructions.
This document outlines a text-based communication protocol for this exact use case, called the _bridge_ protocol.
Should the text-based nature of the protocol turn out to bottleneck the FFI or produce excessive overhead, it needs to be replaced by a more efficient binary-based protocol in the future.

## Medium

The bridge protocol is message-based and therefore has to be carried over any message/datagram-based medium.
To carry the messages over a streaming socket, for instance, a layer/shim needs to be introduced that allows bounded messages to be exchanged.
The medium has to support UTF-8 encoding, as that is what the messages are kept in.

In the context of this project, netstrings as the message containers are used on top of unix domain sockets.
Netstrings use the following format: `<len>:<msg>,`.

## Kinds of Messages

Each message is made up of a message kind and different components, depending on the kind.
The kind is situated at the beginning of the message.
The message kind and the components are separated by newlines.
Currently, there are four kinds of messages: Call (C), Request (R), Send (S), and Error (E), which are described in the following sections.
The messages are kept concise intentionally (e.g. by using abbreviations) to reduce communication and parsing overhead.

### Call

The "Call" (C) message is used to invoke functions or methods on the other side of the FFI and has the following schema:

```
C
<invocation path>
<return value sink>
<parameters>
```

The individual components are defined as follows:

- invocation path: A base64 encoded path to the function or method in the module system of the library. The individual components of the path are separated by dots in the unencoded version. If the path is referring to a method, the last component of the path is the name of the method, separated by a colon.
- return value sink: A uuid that the callee can use as a reference to send the return value to using a "Send" (S) message. If the invocated function or method does not have a return value, the sink still needs to be set so the other side has a chance to receive a potential error value.
- parameters: A newline separated list of the parameters passed to the function or method. Refer to [the section on parameters](#parameters) for more information.

Example:

The following message calls the method `some` on value `bar` located in module `foo`.
The method has two parameters, with the first one being an integer of value `-42` and the second one being a reference to the object with the UUID `"dd1835c3-24ee-44df-b867-71c136e058ca"`.
The return value is supposed to be sent back with the reference `"352b6376-fff5-4dfa-8337-c85f175c349d"` attached as its sink.

Unencoded (just for demonstration purposes, real messages are always encoded):

```
C
foo.bar:some
352b6376-fff5-4dfa-8337-c85f175c349d
i-42
rdd1835c3-24ee-44df-b867-71c136e058ca
```

Encoded:

```
C
Zm9vLmJhcjpzb21l
352b6376-fff5-4dfa-8337-c85f175c349d
i-2a
rdd1835c3-24ee-44df-b867-71c136e058ca
```

### Request

The "Request" (R) message is used to query attributes on objects and has the following schema:

```
R
<parent reference>
<accessor>
<value sink>
```

The individual components are defined as follows:

- parent reference: A UUID that marks the object on which the attribute is accessed.
- accessor: A base64 encoded attribute that is accessed on the parent.
- value sink: A UUID used as a reference in the "Send" message that returns the requested value.

Example:

The following requests the attribute `foo` on the object referred to by `"dd1835c3-24ee-44df-b867-71c136e058ca"`. Further, the sink `"f0b80bf1-9b5a-449f-b2b5-fa07f57c5287"` is specified to allow the sender to identify the returned value via a "Send" message.

Unencoded (just for demonstration purposes, real messages are always encoded):

```
R
dd1835c3-24ee-44df-b867-71c136e058ca
foo
f0b80bf1-9b5a-449f-b2b5-fa07f57c5287
```

Encoded:

```
R
dd1835c3-24ee-44df-b867-71c136e058ca
Zm9v
f0b80bf1-9b5a-449f-b2b5-fa07f57c5287
```

### Send

The "Send" (S) message is used to send values, usually as a response to a "Request" call or to transport a return value of a "Call" invocation.
It has the following schema:

```
S
<reference>
<parameter>
```

The individual components are defined as follows:

- reference: The UUID address that was previously agreed upon as the sink for the "Send" message, for instance by a "Request" or "Call" message.
- A single parameter that serves as the transferred value. Only a positional parameter is supposed to be used here.

Example:

The following "Send" message returns the integer value `42` to the sink `"dd1835c3-24ee-44df-b867-71c136e058ca"`

```
S
dd1835c3-24ee-44df-b867-71c136e058ca
i42
```

### Error

The "Error" (E) message is used to carry error values when a function or method invocated by a "Call" message fails.
It has the following schema:

```
E
<reference>
<parameter>
```

The individual components are defined as follows:

- reference: The UUID address that was previously agreed upon as the sink for the "Send" message in the "Call" message. Instead of the return value, the error value is sent to the same sink.
- A single parameter that serves as the transferred error value. Only a positional parameter is allowed to be used here. The error value does not have to be of a specific error type, but rather can be any value to support a wide range of languages.

Example:

The following "Error" message returns an error object to the sink `"dd1835c3-24ee-44df-b867-71c136e058ca"`

```
E
dd1835c3-24ee-44df-b867-71c136e058ca
r02e4a529-ea4c-4d70-b718-d8db2b883880
```

## Parameters

Each (positional) parameter in a message has two parts: A descriptor and the value.
The value immediately follows the descriptor without a separator.
The descriptor is used to indicate the data type of the parameter.
The following descriptors exist:

- "i": Signed integer. The value is specified in hex to keep the message short. The hex representation is not case sensitive. If there is no sign, the value is assumed to be positive. A sign can be specified, even if the number is positive.
- "f": Float. The value is kept in its decimal form. It might not be the shortest textual representation, but it can easily be parsed by most languages. If there is no sign, the value is assumed to be positive. A sign can be specified, even if the number is positive.
- "b": Boolean. The value is either `0` for `false` or `1` for `true`.
- "s": String. The value is a base64 encoded version of the string.
- "r": Reference. A uuid that refers to an object. When one side of the FFI needs to access attributes on the referenced value, it needs to send a "Request" message to obtain the nested value.

Examples:

```
i2a
f1.337
b1
rdd1835c3-24ee-44df-b867-71c136e058ca
```

Notice how the sizes of the values are not specified anywhere.
This information can be omitted because the caller and the callee are guaranteed to only exchange data of matching types and sizes via code generation of the library stubs on the caller side.
Technically, by that logic, the descriptors could also be omitted, however, they are kept because they don't take up much space and are useful during debugging.

### Named Parameters

A positional parameter can be turned into a named parameter by appending a base64 encoded parameter name after the value, separated by a whitespace character.

Example:

The following parameter called `"age"` is set to the value `42`.

Unencoded (just for demonstration purposes, real messages are always encoded):

```
i42 name
```

Encoded:

```
i2a YWdl
```

Note: In a list of parameters, named parameters are not required to be situated at the end of the parameter list.
It is the responsibility of the receiver of the parameter list to extract the named parameters from the list.
The remaining positional parameters are used in the order that they are defined in.

## UUIDs

The protocol uses UUIDs as references to values.
The UUIDs are kept in version 4 of the RFC 4122 standard.

## Base64 Encoding

There are multiple different ways to encode using base64.
When this specification for the protocol refers to base64 encoding, the following attributes have to be met:

- Standard base64 (not URL-safe), as defined in RFC 4648.
- No padding (e.g. with a trailing `=`).
- The result has to be single line, so no splitting into (e.g. 76 character wide) chunks.

## Newlines

This protocol heavily relies on newlines to separate the components of a message.
A newline is defined as a single `"\n"` character.
Additional characters, such as `"\r"` are not allowed.

## Versioning

This is Version 1 of the bridge protocol.
The messages themselves do not need to be versioned, however.
This is due to the fact that the FFI tool makes sure that only implementations of the same protocol version are communicating with each other via codegen.

# Future work

This protocol may be expanded in the future.
Some aspects that should be addressed are:

- Calling references instead of paths (e.g. a fist class function stored in a variable).
- Memory management (e.g. a "Drop" message instructing the other side that a reference is no longer required).
- Support languages without null safety. This includes different "null" variants, such as `NaN`, `undefined`, etc. In the current form of the protocol, these "special" values are not allowed.
