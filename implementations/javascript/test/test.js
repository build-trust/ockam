const assert = require('assert');
const generate = require('..');

function test(hours, minutes, seconds, expected) {
  assert.equal(generate(), "a key");
}

test()
