// Local fixture endpoint for the HTTP front demo. The governed http source maps
// inputs to the query string, so this answers GET /v1/pets?id=<id> with a small
// JSON record. No external network; started by run.sh.
import { createServer } from "node:http";

const port = Number(process.env.PORT || 8732);

const server = createServer((req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${port}`);
  if (req.method === "GET" && url.pathname === "/v1/pets") {
    const id = url.searchParams.get("id") ?? "unknown";
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
