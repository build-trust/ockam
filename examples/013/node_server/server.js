import { chromium } from 'playwright';
import { URL } from 'url';

(async () => {
    const browserServer = await chromium.launchServer({
        headless: false,
        wsPath: 'browser',
    });

    const wsEndpoint = browserServer.wsEndpoint();
    const url = new URL(wsEndpoint);
    console.log(`ockam zone outlet --relay browser --to localhost:${url.port}`);
    console.log(`WebSocket endpoint: ${wsEndpoint}`);

    console.log("Press Ctrl+C to exit.");
    process.stdin.resume();

    process.on('SIGINT', () => {
        console.log('Ctrl+C detected. Exiting.');
        process.exit(0);
    });


})();