import {
  createFileRegistryStore,
  publishSkillMarkdown,
  type PutVersionOptions,
  type PublishSkillMarkdownOptions,
  type PublishSkillMarkdownResult,
  type RegistrySkill,
  type RegistrySkillVersion,
  type RegistryStore,
} from "@runxhq/runtime-local/sdk";

export { createFileRegistryStore };
export type { RegistrySkillVersion, RegistryStore };

export async function seedRegistrySkill(
  store: RegistryStore,
  markdown: string,
  options: PublishSkillMarkdownOptions = {},
): Promise<RegistrySkillVersion> {
  return (await publishRegistryFixtureSkill(store, markdown, options)).record;
}

export async function publishRegistryFixtureSkill(
  store: RegistryStore,
  markdown: string,
  options: PublishSkillMarkdownOptions = {},
): Promise<PublishSkillMarkdownResult> {
  return await publishSkillMarkdown(store, markdown, options);
}

export async function buildRegistryFixtureVersion(
  markdown: string,
  options: PublishSkillMarkdownOptions = {},
): Promise<RegistrySkillVersion> {
  return await seedRegistrySkill(createMemoryRegistryStore(), markdown, options);
}

export function createMemoryRegistryStore(): RegistryStore {
  const versions = new Map<string, RegistrySkillVersion>();

  return {
    putVersion: async (version: RegistrySkillVersion, options?: PutVersionOptions): Promise<RegistrySkillVersion> => {
      const key = versionKey(version.skill_id, version.version);
      const existing = versions.get(key);
      if (existing && (existing.digest !== version.digest || existing.profile_digest !== version.profile_digest) && !options?.upsert) {
        throw new Error(`Registry version ${version.skill_id}@${version.version} already exists with a different digest.`);
      }
      const stored = existing ? { ...version, created_at: existing.created_at } : version;
      versions.set(key, stored);
      return stored;
    },
    getVersion: async (skillId: string, version?: string): Promise<RegistrySkillVersion | undefined> => {
      const candidates = sortedVersions(Array.from(versions.values()).filter((candidate) => candidate.skill_id === skillId));
      return version ? candidates.find((candidate) => candidate.version === version) : candidates.at(-1);
    },
    listVersions: async (skillId: string): Promise<readonly RegistrySkillVersion[]> =>
      sortedVersions(Array.from(versions.values()).filter((candidate) => candidate.skill_id === skillId)),
    listSkills: async (): Promise<readonly RegistrySkill[]> => {
      const bySkill = new Map<string, RegistrySkillVersion[]>();
      for (const version of versions.values()) {
        bySkill.set(version.skill_id, [...(bySkill.get(version.skill_id) ?? []), version]);
      }
      const skills: RegistrySkill[] = [];
      for (const [skillId, skillVersions] of bySkill.entries()) {
        const sorted = sortedVersions(skillVersions);
        const latest = sorted.at(-1);
        if (latest) {
          skills.push({
            skill_id: skillId,
            owner: latest.owner,
            name: latest.name,
            description: latest.description,
            latest_version: latest.version,
            latest_digest: latest.digest,
            versions: sorted,
          });
        }
      }
      return skills.sort((left, right) => left.skill_id.localeCompare(right.skill_id));
    },
  };
}

function versionKey(skillId: string, version: string): string {
  return `${skillId}@${version}`;
}

function sortedVersions(versions: readonly RegistrySkillVersion[]): readonly RegistrySkillVersion[] {
  return versions
    .slice()
    .sort((left, right) => left.created_at.localeCompare(right.created_at) || left.version.localeCompare(right.version));
}
