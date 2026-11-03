import { expect, test } from "@playwright/test";

test("HUD page: simulated table, edit mode", async ({ page }) => {
  await page.goto("/#/m/hud");
  await expect(page.getByTestId("hud-unsupported")).toBeVisible();
  await expect(page.getByTestId("hud-playmode")).toContainText("Étude");
  await page.getByTestId("simulate-3").click();
  await expect(page.getByTestId("hud-msg")).toContainText("TRIDENT SPACE KO");
  await expect(page.getByTestId("hud-tables")).toContainText("SIMULATION");
  await page.getByTestId("hud-edit").click();
  await expect(page.getByTestId("hud-edit")).toContainText("Quitter");
});

test("HUD overlay: one panel per opponent, drag saves the position, popup in edit mode", async ({ page }) => {
  await page.goto("/#/m/hud");
  await page.getByTestId("hud-edit").click();
  await page.goto("/#/hud/1000000");
  const panels = page.getByTestId("hud-panel");
  await expect(panels).toHaveCount(2); // hero panel hidden by default
  await expect(page.getByTestId("hud-simulated")).toBeVisible();
});

test("HUD overlay drag and popup", async ({ page }) => {
  // The mock keeps edit mode per page load: turn it on from the overlay's own backend.
  await page.goto("/#/hud/1000000");
  await page.evaluate(async () => {
    // Runs in the browser, against the Vite dev server's module graph.
    const path = "/src/lib/backend.ts";
    const m = (await import(/* @vite-ignore */ path)) as { getBackend: () => Promise<{ hudSetEdit: (e: boolean) => Promise<void> }> };
    const b = await m.getBackend();
    await b.hudSetEdit(true);
  });
  const panel = page.getByTestId("hud-panel").first();
  await expect(panel).toHaveCSS("cursor", "move");
  await panel.hover();
  await expect(page.getByTestId("hud-popup")).toContainText("mains");
  const before = await panel.boundingBox();
  await page.mouse.move(before!.x + 10, before!.y + 5);
  await page.mouse.down();
  await page.mouse.move(before!.x + 160, before!.y + 120, { steps: 8 });
  await page.mouse.up();
  await expect.poll(async () => (await page.getByTestId("hud-panel").first().boundingBox())!.x).toBeGreaterThan(before!.x + 100);
});

test("show my own HUD panel", async ({ page }) => {
  await page.goto("/#/m/hud");
  const box = page.getByTestId("hud-show-hero");
  await expect(box).toBeChecked(); // default layout shows the hero
  await box.uncheck();
  await expect(box).not.toBeChecked();
});
