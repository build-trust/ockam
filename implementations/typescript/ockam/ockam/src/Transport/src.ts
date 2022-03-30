import express, {Request, Response, Application} from 'express'
const app:Application = express()
const server = require('http').createServer(app)
import WebSocket from 'ws'

const wss = new WebSocket.Server({ server:server });

wss.on('connection', function connection(ws) {
  console.log('A new client Connected!');
  ws.send('Welcome New Client!');

  ws.on('message', function incoming(message) {
    console.log('received: %s', message);

    wss.clients.forEach(function each(client) {
      if (client !== ws && client.readyState === WebSocket.OPEN) {
        client.send(message);
      }
    });

  });
});


app.get('/', (req: Request, res:Response):void => {
    res.send('Hello from Ockam!!!!')
})

server.listen(3000, () => console.log(`Lisening on port : 3000`))
