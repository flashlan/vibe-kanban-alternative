import { createServer } from 'node:http';

const port = Number.parseInt(process.env.PORT ?? '4000', 10);
const host = process.env.HOST ?? '127.0.0.1';

const server = createServer((request, response) => {
  response.setHeader('Content-Type', 'application/json; charset=utf-8');

  if (request.url === '/health') {
    response.writeHead(200);
    response.end(JSON.stringify({ status: 'ok', service: 'aurapunk-demo-server' }));
    return;
  }

  if (request.url === '/') {
    response.writeHead(200);
    response.end(
      JSON.stringify({
        service: 'aurapunk-demo-server',
        message: 'Ready for an agent workspace',
        version: '0.1.0',
      }),
    );
    return;
  }

  response.writeHead(404);
  response.end(JSON.stringify({ error: 'Not found' }));
});

server.listen(port, host, () => {
  console.log(`AuraPunk demo server listening on http://${host}:${port}`);
});
