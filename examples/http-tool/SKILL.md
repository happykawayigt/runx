---
name: http-tool
description: HTTP front sub-skill; a governed GET against a local fixture endpoint.
source:
  type: http
  url: http://127.0.0.1:8732/v1/pets
  method: GET
  allow_private_network: true
inputs:
  id:
    type: string
    required: true
    description: The pet id; sent as a query parameter.
---
A governed HTTP GET expressed as a first-class `http` source. The runtime maps the
inputs to the query string, runs the call through the governed transport (SSRF and
private-network filtering, header validation, no-redirect, SSL, timeouts), and
seals the response like any other source. There is one governed HTTP path; this
reuses the same transport the registry client and the managed-agent resolver use.

`allow_private_network` is the explicit operator opt-in that lets this reach the
loopback fixture. The default transport blocks private and loopback networks, so
without the opt-in this call is refused (the SSRF guard), not silently allowed.

`examples/http-graph/run.sh` starts the fixture and shows the real response sealed
into the receipt. `http` is a graph-step front, not a top-level runner.
