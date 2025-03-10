# `libsyncrpc`

This is a `NAPI`-based NPM package that provides synchronous IPC/RPC using a
simple line protocol. It uses [`NAPI-RS`](https://napi.rs) under the hood. See
their site for more details as needed.

### Example

```typescript
import { SyncRpcChannel } from "libsyncrpc";

const channel = new SyncRpcChannel("node", "./myscript.js");

channel.registerCallback("callMeMaybe", (method: string, payload: string) => {
    console.log(`method '${method}' invoked 'callMeMaybe' callback`);
    const parsed = JSON.parse(payload);
    parsed.touched = true;
    return JSON.stringify(parsed);
});

const result = channel.requestSync("echo", JSON.stringify({hello: "world"}));

console.log(result); // { hello: "world", touched: true }

const buffer = channel.requestBinarySync("getRawData", "{}");
console.log(buffer); // <Buffer 01 02 03 04 05 06 07 08 09 0a>

// Remember to clean up after yourself!
channel.murderInColdBlood();
```

### Protocol

Requests follow a simple line-based protocol that communicates with the
child process through the child's stdin and stdout streams.

All payloads are assumed to be pre-encoded JSON strings on either end--this
API does not do any of its own JSON encoding/decoding itself. That said, the
data can be any string as long as it doesn't contain a literal `\n` in it.

#### Protocol

The child should handle the following messages through its `stdin`:

* `request\t<method>\t<payload>\n`: a request to the child with the
  given JSON `<payload>`, with `<method>` as the method name. The child
  should send back any number of `call` messages and close the request
  with either a `response` or `error` message.
* `request-bin\t<method>\t<payload>\n`: a request for binary data from the child.
  The child should send back a `response-bin` message with the binary data.
* `call-response\t<name>\t<payload>\n`: a response to a `call` message
  that the child previously sent. The `<payload>` is the encoded result
  from invoking the JavaScript callback associated with it. If the
  callback errors
* `call-error\t<name>\t<message>\n`: informs the child that an error
  occurred. The `<message>` will simply be the stringified error, not
  necessarily in JSON format. This method will also throw an error after
  sending this message to its child and terminate the request call.

The channel handles the following messages from the child's `stdout`:

* `response\t<method>\t<payload>\n`: a response to a request that the
  call was for. `<payload>` will be the call's return value, and should
  be a JSON-encoded string. `<method>` MUST match the `request`
  message's `<method>` argument.
* `response-bin\t<method>\t<size>\t<payload>\n`: a binary response to a request.
  `<method>` MUST match the `request-bin` message's `<method>` argument. `<size>` is
  the content-length in bytes of the binary `<payload>`. The library will validate
  that the payload size matches the specified size. This format allows the payload
  to contain any binary data, including newline characters.
* `error\t<method>\t<message>\n`: a response that denotes some error
  occurred while processing the request on the child side. The
  `<message>` will be the stringified error, not necessarily in JSON
  format. It will be used as the error message that this method will
  throw (terminating the request). `<method>` MUST match the `request`
  message's `<method>` argument.
* `call\t<name>\t<payload>\n`: a request to invoke a pre-registered
  JavaScript callback (see `registerCallback`). `<name>` is the name of
  the callback, and `<payload>` is the JSON-encoded string that the
  callback will be called with. The child should then listen for
  `call-response` and `call-error` messages.

### Building

1. [Install Rust](https://www.rust-lang.org/tools/install) (note that you may need VS C++ Buil Tools when prompted).
2. [Install Node.js](https://nodejs.org/en/download)
3. Clone this repository
4. `cd <repo location> && npm i`
5. `npm run build`

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