import { describe, expect, it } from "vitest";
import {
  createLocalFindingsClient,
  type NewFinding,
} from "../../src/lib/findings";

function memoryStore() {
  const m = new Map<string, string>();
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => {
      m.set(k, v);
    },
    raw: m,
  };
}

const sample: NewFinding = {
  x: 10,
  y: 20,
  ra_deg: 133.79,
  dec_deg: -7.24,
  band: "b2",
  epoch: "B",
  kind: "comet",
  note: "moves north-east",
  frame_id: "f-1",
};

describe("local findings client", () => {
  it("adds, lists and removes findings", async () => {
    const store = memoryStore();
    const c = createLocalFindingsClient(store, "k");
    const f = await c.add(sample);
    expect(f.id).not.toBe("");
    expect(f.kind).toBe("comet");
    expect(await c.list()).toHaveLength(1);
    // A second client over the same storage sees the persisted finding.
    expect(await createLocalFindingsClient(store, "k").list()).toHaveLength(1);
    await c.remove(f.id);
    expect(await c.list()).toEqual([]);
  });
  it("rejects invalid findings", async () => {
    const c = createLocalFindingsClient(memoryStore(), "k");
    await expect(c.add({ ...sample, x: -1 })).rejects.toThrow();
    await expect(c.add({ ...sample, ra_deg: 360 })).rejects.toThrow();
    await expect(
      c.add({ ...sample, kind: "star" as NewFinding["kind"] }),
    ).rejects.toThrow();
  });
  it("ignores corrupt storage", async () => {
    const store = memoryStore();
    store.setItem("k", "{not json");
    const c = createLocalFindingsClient(store, "k");
    expect(await c.list()).toEqual([]);
    store.setItem("k", JSON.stringify([{ id: 1 }]));
    expect(await c.list()).toEqual([]);
  });
  it("keeps findings in memory when storage writes fail", async () => {
    const store = memoryStore();
    const c = createLocalFindingsClient(
      {
        getItem: store.getItem,
        setItem: () => {
          throw new Error("quota");
        },
      },
      "k",
    );
    const f1 = await c.add(sample);
    await c.add({ ...sample, x: 11 });
    expect(await c.list()).toHaveLength(2);
    await c.remove(f1.id);
    expect((await c.list()).map((f) => f.x)).toEqual([11]);
  });
  it("works without storage", async () => {
    const c = createLocalFindingsClient(null);
    await c.add(sample);
    expect(await c.list()).toHaveLength(1);
  });
});
