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
