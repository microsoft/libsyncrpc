import { Bench } from 'tinybench';
import { SyncRpcChannel } from "./index.js";

const channel = new SyncRpcChannel("node", ["./echo.mjs"]);
const bench = new Bench();

bench
    // .add("baseline function call", () => {
    //     exampleFun("echo", '"hello"');
    // })
    .add('simple echo request', () => {
        channel.requestSync("echo", '"hello"');
    });

await bench.warmup(); // make results more reliable, ref: https://github.com/tinylibs/tinybench/pull/50
await bench.run();

console.table(bench.table());

process.exit(0);

function exampleFun(method, payload) {
    `response\t${method}\t${payload}`;
}