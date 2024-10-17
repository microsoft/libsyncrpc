import test from 'ava'

import { SyncRpcChannel } from '../index.js';

test("should be able to send a message and get a response, synchronously.",
  t => {
    const channel = makeChannel();
    const response = channel.requestSync("echo", '"hello"');
    t.is(response, '"hello"');
  }
);

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
  t.is(response, '"onetwothree"');
});

function makeChannel() {
  return new SyncRpcChannel("cargo", ["run", "--release", "--example", "socket_child"]);
}