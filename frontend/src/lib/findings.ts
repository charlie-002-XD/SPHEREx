// Findings (moving-object reports) marked on the epoch comparator. The API
// contract (shared/openapi-contract.json) has no findings endpoint yet, so
// the default client persists to localStorage behind the FindingsClient
// interface; a server-backed client can replace it without touching the UI.
import { z } from "zod";

export const FINDING_KINDS = ["asteroid", "comet", "planet"] as const;
export type FindingKind = (typeof FINDING_KINDS)[number];

export const FINDING_KIND_LABELS: Record<FindingKind, string> = {
  asteroid: "Asteroid",
  comet: "Comet",
  planet: "Planet",
};

export const NOTE_MAX = 500;

export const findingSchema = z.object({
  id: z.string().min(1),
  x: z.number().int().min(0),
  y: z.number().int().min(0),
  ra_deg: z.number().min(0).lt(360).nullable(),
  dec_deg: z.number().min(-90).max(90).nullable(),
  band: z.string().min(1),
  epoch: z.enum(["A", "B"]),
  kind: z.enum(FINDING_KINDS),
  note: z.string().max(NOTE_MAX),
  frame_id: z.string(),
  created_at: z.string(),
});
export type Finding = z.infer<typeof findingSchema>;

export const newFindingSchema = findingSchema.omit({
  id: true,
  created_at: true,
});
export type NewFinding = z.infer<typeof newFindingSchema>;

export interface FindingsClient {
  readonly kind: "local" | "remote";
  list(): Promise<Finding[]>;
  add(f: NewFinding): Promise<Finding>;
  remove(id: string): Promise<void>;
}

export const FINDINGS_STORAGE_KEY = "spherex-blink.findings.v1";

let fallbackSeq = 0;
function newId(): string {
  const c = globalThis.crypto;
  if (c !== undefined && typeof c.randomUUID === "function") {
    return c.randomUUID();
  }
  fallbackSeq += 1;
  return `f-${Date.now().toString(36)}-${fallbackSeq.toString(36)}`;
}

type KeyValueStore = Pick<Storage, "getItem" | "setItem">;

function safeStorage(): KeyValueStore | null {
  try {
    return typeof window !== "undefined" ? window.localStorage : null;
  } catch {
    return null; // storage disabled (privacy mode, sandboxed iframe)
  }
}

// Unreadable or corrupt stored data is ignored (not thrown) and kept in
// place until the next successful write, so a bad entry never breaks the UI.
export function createLocalFindingsClient(
  store: KeyValueStore | null = safeStorage(),
  key = FINDINGS_STORAGE_KEY,
): FindingsClient {
  let memory: Finding[] = [];
  // After a failed write the in-memory copy is newer than storage.
  let storageStale = false;

  const read = (): Finding[] => {
    if (store === null || storageStale) return memory;
    try {
      const raw = store.getItem(key);
      if (raw === null) return [];
      const parsed = z.array(findingSchema).safeParse(JSON.parse(raw));
      return parsed.success ? parsed.data : [];
    } catch {
      return [];
    }
  };

  const write = (list: Finding[]): void => {
    memory = list;
    if (store === null) return;
    try {
      store.setItem(key, JSON.stringify(list));
      storageStale = false;
    } catch {
      storageStale = true;
      // quota exceeded or storage disabled: keep the in-memory copy
    }
  };

  return {
    kind: "local",
    list: () => Promise.resolve(read()),
    add: (input) => {
      const valid = newFindingSchema.safeParse(input);
      if (!valid.success) {
        return Promise.reject(new Error("Invalid finding."));
      }
      const finding: Finding = {
        ...valid.data,
        id: newId(),
        created_at: new Date().toISOString(),
      };
      write([...read(), finding]);
      return Promise.resolve(finding);
    },
    remove: (id) => {
      write(read().filter((f) => f.id !== id));
      return Promise.resolve();
    },
  };
}
