import { render, screen, waitFor } from "@testing-library/react";
import { App } from "@/App";
import { setBackend } from "@/lib/backend";
import { createMockBackend } from "@/lib/backend.mock";

describe("App shell", () => {
  it("builds tiles and sidebar from the manifests", async () => {
    setBackend(createMockBackend());
    render(<App />);
    await waitFor(() => expect(screen.getAllByTestId("module-tile")).toHaveLength(10));
    const sidebar = await screen.findByTestId("sidebar-modules");
    expect(sidebar.querySelectorAll("a")).toHaveLength(10);
  });
});

describe("fair-play lock", () => {
  it("study modules show the lock while playing", async () => {
    const { usePlayMode } = await import("@/stores/playMode");
    setBackend(createMockBackend());
    window.location.hash = "#/m/solver";
    render(<App />);
    usePlayMode.getState().set(true);
    // usePlayModeSync sets it back from the backend (false) on mount; force it again after.
    await waitFor(() => expect(screen.getAllByTestId("coming-soon").length + 0).toBeGreaterThanOrEqual(0));
    usePlayMode.getState().set(true);
    await waitFor(() => expect(screen.getByTestId("study-lock")).toBeTruthy());
    usePlayMode.getState().set(false);
    await waitFor(() => expect(screen.queryByTestId("study-lock")).toBeNull());
    window.location.hash = "";
  });
});
