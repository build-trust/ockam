const { loadBinding } = require('@node-rs/helper')
module.exports = loadBinding(__dirname, 'nodejs_native', '@ockam/nodejs_native')
