import fs from "node:fs";

import { foldEventPage } from "../state.mjs";

const inputs = readInputs();
const result = foldEventPage({
  projection: inputs.projection,
  events: inputs.events,
  afterVersion: inputs.after_version,
  streamVersion: inputs.stream_version,
});

process.stdout.write(`${JSON.stringify({ projection: result }, null, 2)}\n`);

function readInputs() {
  const raw = process.env.RUNX_INPUTS_PATH
    ? fs.readFileSync(process.env.RUNX_INPUTS_PATH, "utf8")
    : process.env.RUNX_INPUTS_JSON || "{}";
  return JSON.parse(raw);
}
