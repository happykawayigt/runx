import { describe, expect, it } from "vitest";

import { cliPackage } from "../packages/cli/src/index.js";
import { parserPackage } from "../packages/cli/src/cli-parser/index.js";

describe("bootstrap workspace", () => {
  it("wires trusted-kernel package exports", () => {
    expect([cliPackage, parserPackage]).toEqual([
      "@runxhq/cli",
      "@runxhq/cli/parser",
    ]);
  });
});
