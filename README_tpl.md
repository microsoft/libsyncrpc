# `@typescript/libsyncrpc`

This is a `NAPI`-based NPM package that provides synchronous IPC/RPC using a
simple line protocol. It uses [`NAPI-RS`](https://napi.rs) under the hood. See
their site for more details as needed.

## Example

```typescript
import { SyncRpcChannel } from "@typescript/libsyncrpc";

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

__DTSMD_PLACEHOLDER__
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
