---
name: http-graph
description: HTTP front example; a graph whose step turns a governed HTTP call into a sealed receipt.
---
# HTTP graph

A single-step graph that drives the `http` sub-skill. The runtime routes the graph
step's `http` source through the source-adapter registry to the governed HTTP
adapter, which maps the step inputs to the request, runs it through the governed
`runtime_http` transport, and seals the response.

This is the keystone call-out front: a new HTTP integration is "point at the
endpoint, map inputs, govern it," not a hand-rolled script or a bespoke server.
Run the inline harness with `runx harness examples/http-graph`, or
`examples/http-graph/run.sh` to see the real response sealed against a local
fixture endpoint.
