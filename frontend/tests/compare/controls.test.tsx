// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CompareControls } from "../../src/components/compare/CompareControls";

afterEach(cleanup);

describe("CompareControls", () => {
  it("Play in slider mode asks to start blinking even if playback was on", () => {
    const onPlaying = vi.fn();
    render(
      <CompareControls
        mode="slider"
        onMode={() => {}}
        playing
        onPlaying={onPlaying}
        intervalMs={500}
        onInterval={() => {}}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Play blink" }));
    expect(onPlaying).toHaveBeenCalledWith(true);
  });

  it("keeps play and speed enabled in slider mode", () => {
    render(
      <CompareControls
        mode="slider"
        onMode={() => {}}
        playing={false}
        onPlaying={() => {}}
        intervalMs={500}
        onInterval={() => {}}
      />,
    );
    const play = screen.getByRole("button", { name: "Play blink" });
    const speed = screen.getByRole("slider", { name: "Blink interval" });
    expect((play as HTMLButtonElement).disabled).toBe(false);
    expect((speed as HTMLInputElement).disabled).toBe(false);
  });
});
