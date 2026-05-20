#!/usr/bin/env node
const fallback = process.env.RUNX_JS_BIN || "npm exec runx";
console.error(fallback);
