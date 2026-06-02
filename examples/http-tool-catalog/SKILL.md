---
name: http-tool-catalog
description: HTTP tool example; a graph step invokes a governed http tool through the catalog and seals it.
---
# HTTP tool via the catalog

A single-step graph whose step references a governed **http tool** by ref
(`tool: demo.pet_get`). The runtime resolves the local tool, sees its `http`
source, and routes it through the governed HTTP adapter, sealing the response.
This is the tool-path counterpart to `examples/http-graph` (which drives an http
source as a graph step): here an agent's *tool* is a governed HTTP call.

The tool also uses a `{id}` path placeholder, so it demonstrates URL path
templating against the fixture. Run `examples/http-tool-catalog/run.sh`.
