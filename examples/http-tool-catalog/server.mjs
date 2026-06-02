// Local fixture endpoint for the HTTP tool demo. The http tool's url is
// path-templated (/v1/pets/{id}), so this answers GET /v1/pets/<id> with a small
// JSON record. No external network; started by run.sh.
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
