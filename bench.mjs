import { Bench } from 'tinybench';
import { SyncRpcChannel } from "./index.js";

const rustChannel = new SyncRpcChannel("cargo", ["run", "--release", "--example", "socket_child"])
const nodeChannel = new SyncRpcChannel("node", ["./echo.mjs"])
const bench = new Bench();

bench
    // .add("baseline function call", () => {
    //     exampleFun("echo", '"hello"');
    // })
    .add('simple echo request to Node child', () => {
        nodeChannel.requestSync("echo", '"hello"');
    })
    .add('simple echo request to Rust child', () => {
        rustChannel.requestSync("echo", '"hello"');
    });

await bench.warmup(); // make results more reliable, ref: https://github.com/tinylibs/tinybench/pull/50
await bench.run();

console.table(bench.table());

process.exit(0);