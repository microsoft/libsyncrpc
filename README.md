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

Requests follow a MessagePack-based "tuple"/array protocol with 3 items:
`(<type>, <name>, <payload>)`. All items are binary arrays of 8-bit
integers, including the `<type>` and `<name>`, to avoid unnecessary
encoding/decoding at the protocol level.

For specific message types and their corresponding protocol behavior, please
see `MessageType` below.

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

```ts
requestBinarySync(method: string, payload: Uint8Array): Uint8Array;
```

#### SyncRpcChannel.prototype.registerCallback (method)

Registers a JavaScript callback that the child can invoke before
completing a request. The callback will receive a string name and a string
payload as its arguments and should return a string as its result.

There is currently no `Uint8Array`-only equivalent to this functionality.

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
