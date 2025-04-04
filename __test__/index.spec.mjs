import { dirname, join } from "node:path";
import { fileURLToPath } from 'node:url';

import test from 'ava'

// Earlier versions of node@20 don't have `import.meta.dirname`.
const __dirname = import.meta.dirname || dirname(fileURLToPath(import.meta.url));

import { SyncRpcChannel } from '../index.js';

test("should be able to send a message and get a response, synchronously.", t => {
  const channel = makeChannel();
  const response = channel.requestSync("echo", '"hello"');
  t.is(response, '"hello"');
  channel.close();
});

test("can register a callback that will be requested by the child process before returning", t => {
  const channel = makeChannel();
  channel.registerCallback("echo", (_name, message) => message);
  const response = channel.requestSync("callback-echo", '"hello"');
  t.is(response, '"hello"');
});

test("callbacks are handled in the order in which they're requested", t => {
  const channel = makeChannel();
  channel.registerCallback("one", (_name, _message) => "one");
  channel.registerCallback("two", (_name, _message) => "two");
  channel.registerCallback("three", (_name, _message) => "three");
  const response = channel.requestSync("concat", "");
  t.is(response, "onetwothree");
  channel.close();
});

test("throws if the child responds with an error", t => {
  const channel = makeChannel();
  t.throws(() => {
    channel.requestSync("error", "");
  }, { code: "GenericFailure", message: '"something went wrong"' });
  channel.close();
});

test("throws if a callback throws", t => {
  const channel = makeChannel();
  channel.registerCallback("throw", () => { throw new Error("callback error") });
  t.throws(() => {
    channel.requestSync("throw", "");
  }, { code: "GenericFailure", message: /callback error/ });
  channel.close();
});

// function makeChannel() {
//   return new SyncRpcChannel("cargo", ["run", "--release", "--example", "socket_child"]);
// }

function makeChannel() {
  return new SyncRpcChannel("node", [join(__dirname, "../echo.mjs")]);
}