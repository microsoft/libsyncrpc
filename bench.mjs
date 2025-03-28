import { Bench } from 'tinybench';
import { SyncRpcChannel } from "./index.js";

const rustChannel = new SyncRpcChannel("cargo", ["run", "--release", "--example", "socket_child"])
const nodeChannel = new SyncRpcChannel("node", ["./echo.mjs"])
const bench = new Bench();

const ENCODER = new TextEncoder();
const smallMsg = ENCODER.encode('"hello"');
const bigMsg = new Uint8Array(1024 * 1024);
const hugeMsg = new Uint8Array(1024 * 1024 * 1024);
const smallStr = '"hello"';
const bigStr = "x".repeat(1024 * 1024);

bench
    .add('simple echo request to Rust child', () => {
        rustChannel.requestSync("echo", smallStr);
    })
    .add('simple echo request to Rust child with a bigger 1MiB message', () => {
        rustChannel.requestSync("echo", bigStr);
    })
    .add('simple binary echo request to Rust child', () => {
        rustChannel.requestBinarySync("echo", smallMsg);
    })
    .add('simple binary echo request to Rust child with a bigger 1MiB message', () => {
        rustChannel.requestBinarySync("echo", bigMsg);
    })
    .add('simple binary echo request to Rust child with an even bigger 1GiB message', () => {
        rustChannel.requestBinarySync("echo", hugeMsg);
    })
    .add('simple echo request to Node child', () => {
        nodeChannel.requestSync("echo", smallStr);
    })
    .add('simple echo request to Node child with a bigger 1MiB message', () => {
        nodeChannel.requestSync("echo", bigStr);
    })
    .add('simple binary echo request to Node child', () => {
        nodeChannel.requestBinarySync("echo", smallMsg);
    })
    .add('simple binary echo request to Node child with a bigger 1MiB message', () => {
        nodeChannel.requestBinarySync("echo", bigMsg);
    })
    .add('js noop baseline', () => {
        noopjs(smallMsg);
    })
    .add('js noop baseline with a bigger 1MiB message', () => {
        noopjs(bigMsg);
    });

await bench.warmup(); // make results more reliable, ref: https://github.com/tinylibs/tinybench/pull/50
await bench.run();

function noopjs(x) {
    return x;
}

console.table(bench.table());

rustChannel.close();
process.exit(0);