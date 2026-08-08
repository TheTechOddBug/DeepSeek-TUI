#!/usr/bin/env node

const { runCodeWhale } = require("../scripts/run");

process.stderr.write("codewhale-tui: deprecated alias to `codewhale` (single binary since v0.9.5). Use `codewhale` instead.\n");
runCodeWhale().catch((error) => {
  console.error("Failed to start codewhale:", error.message);
  process.exit(1);
});
