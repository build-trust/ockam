# Browser agent example

## Setup for local headed browsing session

### Tab 1
```
ockam
```

### Tab 2
```
cd node_server
npm install
npx playwright install
node server.js
```
This will start a local playwright session in headed mode.

It will also emit the command to create an outlet, copy that command.

### Tab 3
- wait until the `ockam` command from tab 1 has started the zone
- paste and run the command from step 2

## next steps

Tab 2 and tab 3 can be left as is.

The main `ockam` session from tab 1 will work after either k8s restarts the container or after a manual restart.

