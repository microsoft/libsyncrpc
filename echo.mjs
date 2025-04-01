import { on, once } from "node:events";
import { PackrStream, UnpackrStream } from "msgpackr";
import { MessageType } from './index.js';

const unpackStream = new UnpackrStream();
const packStream = new PackrStream();
process.stdin.pipe(unpackStream);
packStream.pipe(process.stdout);

const DECODER = new TextDecoder();
const ENCODER = new TextEncoder();

let pendingCallResponse = false;

for await (const msgs of on(unpackStream, "data")) {
    for (const [ty, binName, payload] of msgs) {
        const name = DECODER.decode(binName);
        top: switch (ty) {
            case MessageType.Request:
                switch (name) {
                    case "echo":
                        await write(MessageType.Response, name, payload);
                        break top;
                    case "callback-echo":
                        const resPayload = await call("echo", payload);
                        await write(MessageType.Response, name, resPayload);
                        break top;
                    case "concat":
                        const one = await call("one", "1");
                        const two = await call("two", "2");
                        const three = await call("three", "3");
                        const ret = new Uint8Array(one.length + two.length + three.length);
                        ret.set(one);
                        ret.set(two, one.length);
                        ret.set(three, one.length + two.length);
                        await write(MessageType.Response, name, ret);
                        break top;
                    case "error":
                        await write(MessageType.Error, name, "\"something went wrong\"");
                        break top;
                    case "throw":
                        await write(MessageType.Call, name, "");
                        pendingCallResponse = true;
                        const [[resTy, resName]] = await once(unpackStream, "data");
                        const decResName = DECODER.decode(resName);
                        if (resTy != MessageType.CallError || decResName) {
                            throw new Error(`Unexpected response: (${resTy}) ${decResName}`);
                        }
                        // Do nothing
                        break top;
                }
                break;
            case MessageType.CallResponse:
                if (pendingCallResponse) {
                    pendingCallResponse = false;
                } else {
                    throw new Error("Unexpected CallResponse");
                }
                break;
            default:
                throw new Error(`Unexpected message: (${ty}) ${name}`)
        }
    }
}

async function write(ty, name, payload) {
    const ret = await new Promise((resolve, reject) => {
        packStream.write([ty, bin(name), bin(payload)], (x) => x ? reject(x) : resolve());
    });
    return ret;
}

async function call(name, payload) {
    const waiter = once(unpackStream, "data");
    await write(MessageType.Call, name, payload);
    pendingCallResponse = true;
    const [[resTy, resName, resPayload]] = await waiter;
    if (resTy != MessageType.CallResponse) {
        throw new Error(`Expected CallResponse but got ${resTy}`);
    }
    const decResName = DECODER.decode(resName);
    if (decResName != name) {
        throw new Error(`Unexpected response: ${decResName}`);
    }
    return resPayload;
}

function bin(input) {
    return typeof input === "string" ? ENCODER.encode(input) : input;
}