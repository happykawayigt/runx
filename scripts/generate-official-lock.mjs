#!/usr/bin/env node

import { access, readdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { parseRunnerManifestYaml, parseSkillMarkdown, validateRunnerManifest, validateSkill } from "@runxhq/core/parser";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDir, "..");
const skillsRoot = path.join(workspaceRoot, "skills");
const outputPath = path.join(workspaceRoot, "packages", "cli", "src", "official-skills.lock.json");

const entries = [];
for (const entry of (await readdir(skillsRoot, { withFileTypes: true })).sort((left, right) => left.name.localeCompare(right.name))) {
  if (!entry.isDirectory()) continue;
  const skillDir = path.join(skillsRoot, entry.name);
  const profilePath = path.join(skillDir, "X.yaml");
  try {
    await access(path.join(skillDir, "SKILL.md"));
    await access(profilePath);
  } catch {
    continue;
  }
  const markdown = await readFile(path.join(skillDir, "SKILL.md"), "utf8");
  const profileDocument = await readFile(profilePath, "utf8");
  const record = buildOfficialSkillLockRecord(markdown, profileDocument);
  entries.push({
    skill_id: record.skill_id,
    version: record.version,
    digest: record.digest,
  });
}

await writeFile(outputPath, `${JSON.stringify(entries, null, 2)}\n`, "utf8");

function buildOfficialSkillLockRecord(markdown, profileDocument) {
  const raw = parseSkillMarkdown(markdown);
  const skill = validateSkill(raw, { mode: "strict" });
  const manifest = validateRunnerManifest(parseRunnerManifestYaml(profileDocument));
  if (manifest.skill && manifest.skill !== skill.name) {
    throw new Error(`Runner manifest skill '${manifest.skill}' does not match skill '${skill.name}'.`);
  }

  const digest = createHash("sha256").update(markdown).digest("hex");
  const profileDigest = createHash("sha256").update(profileDocument).digest("hex");
  const versionSeed = createHash("sha256")
    .update(JSON.stringify({
      markdown_digest: digest,
      profile_digest: profileDigest,
    }))
    .digest("hex");
  return {
    skill_id: `runx/${slugifyOfficialSkillName(skill.name)}`,
    version: `sha-${versionSeed.slice(0, 12)}`,
    digest,
  };
}

function slugifyOfficialSkillName(value) {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  if (!slug) {
    throw new Error("Official skill names cannot produce an empty registry slug.");
  }
  return slug;
}
