// Local fixture endpoint for the OpenAPI front demo. Serves the pets API the
// spec describes so the adapter makes a real local call with no external
// network. Started by run.sh.
import { createServer } from "node:http";

const port = Number(process.env.PORT || 8732);

const server = createServer((req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${port}`);
  const match = url.pathname.match(/^\/v1\/pets\/([^/]+)$/);
  if (req.method === "GET" && match) {
    const id = decodeURIComponent(match[1]);
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify({ id, name: `pet-${id}`, species: "cat" }));
    return;
  }
  res.writeHead(404, { "content-type": "application/json" });
  res.end(JSON.stringify({ error: "not found" }));
});

server.listen(port, "127.0.0.1", () => {
  process.stdout.write(`pets fixture listening on ${port}\n`);
});
