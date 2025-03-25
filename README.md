# `libsyncrpc`

This is a `NAPI`-based NPM package that provides synchronous IPC/RPC using a
simple line protocol. It uses [`NAPI-RS`](https://napi.rs) under the hood. See
their site for more details as needed.

## Example

```typescript
import { SyncRpcChannel } from "libsyncrpc";

const channel = new SyncRpcChannel("node", "./myscript.js");
const DECODER = new TextDecoder();

channel.registerCallback("callMeMaybe", (method: string, payload: string) => {
    console.log(`method '${method}' invoked 'callMeMaybe' callback`);
    const parsed = JSON.parse(payload);
    parsed.touched = true;
    return JSON.stringify(parsed);
});

const result = channel.requestSync("echo", JSON.stringify({hello: "world"}));

console.log(result); // { hello: "world", touched: true }

// Remember to clean up after yourself!
channel.close();
```

## API

### SyncRpcChannel (exported class)

A synchronous RPC channel that allows JavaScript to synchronously call out
to a child process and get a response over a line-based protocol,
including handling of JavaScript-side callbacks before the call completes.

#### Protocol

Requests follow a simple delimiter-and-size-based protocol that communicates
with the child process through the child's stdin and stdout streams.

All payloads are assumed to be pre-encoded JSON strings on either end--this API
does not do any of its own JSON or even string encoding/decoding itself. it.

The child should handle the following messages through its `stdin`. In all
below examples, `<payload-size>` is a 4-byte sequence representing an unsigned
32-bit integer. The following `<payload>` will be that many bytes long. Each
message ends once the payload ends. The payload may be interpreted in
different ways depending on the message, for example as raw binary data or a
UTF-8 string. All other values (`<name>`, `<method>`, etc) are expected to be
UTF-8-encoded bytes.

- `request	<method>	<payload-size><payload>`: a request to the child with the
  given raw byte `<payload>`, with `<method>` as the method name. The child should
  send back any number of `call` messages and close the request with either a
  `response` or `error` message.
- `call-response	<name>	<payload-size><payload>`: a response to a `call`
  message that the child previously sent. The `<payload>` is the return value
  from invoking the JavaScript callback associated with it. If the callback
  errors, `call-error` will be sent to the child.
- `call-error	<name>	<payload-size><payload>`: informs the child that an error
  occurred. The `<payload>` will be the binary representation of the stringified
  error, as UTF-8 bytes, not necessarily in JSON format. The method linked to this
  message will also throw an error after sending this message to its child and
  terminate the request call.

The channel handles the following messages from the child's `stdout`:

- `response	<method>	<payload-size><payload>`: a response to a request that the
  call was for. `<method>` MUST match the `request`
  message's `<method>` argument.
- `error	<method>	<payload-size><payload>`: a response that denotes some error
  occurred while processing the request on the child side. The `<payload>` will
  simply be the binary representation of the stringified error, as UTF-8 bytes,
  not necessarily in JSON format. The method associated with this call will also
  throw an error after receiving this message from the child.
- `call	<name>	<payload-size><payload>`: a request to invoke a pre-registered
  JavaScript callback (see `registerCallback`). `<name>` is the name of the
  callback, and `<payload>` is an encoded UTF-8 string that the callback will be
  called with. The child should then listen for `call-response` and `call-error`
  messages.

```ts
declare class SyncRpcChannel {
  constructor(exe: string, args: Array<string>);
  requestSync(method: string, payload: string): string;
  requestBinarySync(method: string, payload: Uint8Array): Uint8Array;
  registerCallback(
    name: string,
    callback: (name: string, payload: string) => string,
  ): void;
  close(): void;
}
```

#### SyncRpcChannel (constructor)

Constructs a new `SyncRpcChannel` by spawning a child process with the
given `exe` executable, and a given set of `args`.

```ts
constructor(exe: string, args: Array<string>);
```

#### SyncRpcChannel.prototype.requestSync (method)

Send a request to the child process and wait for a response. The method
will not return, synchronously, until a response is received or an error
occurs.

This method will take care of encoding and decoding the binary payload to
and from a JS string automatically and suitable for smaller payloads.

For details on the protocol, refer to `README.md`.

```ts
requestSync(method: string, payload: string): string;
```

#### SyncRpcChannel.prototype.requestBinarySync (method)

Send a request to the child process and wait for a response. The method
will not return, synchronously, until a response is received or an error
occurs.

Unlike `requestSync`, this method will not do any of its own encoding or
decoding of payload data. Everything will be as sent/received through the
underlying protocol.

For details on the protocol, refer to `README.md`.

```ts
requestBinarySync(method: string, payload: Uint8Array): Uint8Array;
```

#### SyncRpcChannel.prototype.registerCallback (method)

Registers a JavaScript callback that the child can invoke before
completing a request. The callback will receive a string name and a string
payload as its arguments and should return a string as its result.

There is currently no Uint8Array-only equivalent to this functionality.

If the callback throws, an it will be handled appropriately by
`requestSync` and the child will be notified.

```ts
registerCallback(name: string, callback: (name: string, payload: string) => string): void;
```

#### SyncRpcChannel.prototype.close (method)

```ts
close(): void;
```

### Building

1. [Install Rust](https://www.rust-lang.org/tools/install) (note that you may need VS C++ Buil Tools when prompted).
2. [Install Node.js](https://nodejs.org/en/download)
3. Clone this repository
4. `cd <repo location> && npm i`
5. `npm run build` (for production/release build), or `npm run build:debug` (for debug build)

### Benchmarking

Simply run `npm run bench`. It will test against both a Node-based child
process and a Rust-based one, using the same protocol.

### Developing

`rust-analyzer` is the Rust language server you want. It's available pretty
much on everything.

For vscode, you may want to add the following to `settings.json`:

```json
{
  "rust-analyzer.procMacro.ignored": { "napi-derive": ["napi"] }
}
```

## Contributing

This project welcomes contributions and suggestions.  Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit [Contributor License Agreements](https://cla.opensource.microsoft.com).

When you submit a pull request, a CLA bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., status check, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

## Trademarks

This project may contain trademarks or logos for projects, products, or services. Authorized use of Microsoft
trademarks or logos is subject to and must follow
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.
Any use of third-party trademarks or logos are subject to those third-party's policies.
