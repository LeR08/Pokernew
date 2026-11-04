import { expect, test } from "@playwright/test";

test("HUD page: detected table, per-table edit and hide", async ({ page }) => {
  await page.goto("/#/m/hud");
  await expect(page.getByTestId("hud-unsupported")).toBeVisible();
  await expect(page.getByTestId("hud-playmode")).toContainText("Étude");
  const row = page.getByTestId("hud-table");
  await expect(row).toHaveCount(1);
  await expect(row).toContainText("Winamax TRIDENT SPACE KO(1195602081)(#0053)");
  const edit = page.getByTestId("hud-table-edit");
  await expect(edit).toContainText("Modifier le HUD de cette table");
  await edit.click();
  await expect(edit).toContainText("Terminer");
  await edit.click();
  await expect(edit).toContainText("Modifier le HUD de cette table");
  const hide = page.getByTestId("hud-table-hide");
  await expect(hide).toContainText("Masquer");
  await hide.click();
  await expect(hide).toContainText("Afficher");
  await page.getByTestId("hud-edit").click();
  await expect(page.getByTestId("hud-edit")).toContainText("Quitter");
});

test("HUD overlay: one panel per opponent, no edit bar outside edit mode", async ({ page }) => {
  await page.goto("/#/hud/1");
  await expect(page.getByTestId("hud-panel")).toHaveCount(2); // the mock layout hides the hero
  await expect(page.getByTestId("hud-edit-bar")).toHaveCount(0);
});

test("HUD overlay: per-table edit, drag, popup, done", async ({ page }) => {
  // Edit mode is turned on from the HUD module, then the overlay route is opened in the same page.
  await page.goto("/#/m/hud");
  await page.getByTestId("hud-table-edit").click();
  await expect(page.getByTestId("hud-table-edit")).toContainText("Terminer");
  await page.goto("/#/hud/1");
  await expect(page.getByTestId("hud-panel")).toHaveCount(2);
  await expect(page.getByTestId("hud-edit-bar")).toBeVisible();
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
  await page.getByTestId("hud-edit-done").click();
  await expect(page.getByTestId("hud-edit-bar")).toHaveCount(0);
});

test("show my own HUD panel", async ({ page }) => {
  await page.goto("/#/m/hud");
  const box = page.getByTestId("hud-show-hero");
  await expect(box).toBeEnabled();
  await expect(box).toBeChecked(); // default layout shows the hero
  await box.uncheck();
  await expect(box).not.toBeChecked();
});

test("HUD overlay: panel size and table format in edit mode", async ({ page }) => {
  await page.goto("/#/m/hud");
  await page.getByTestId("hud-table-edit").click();
  await page.goto("/#/hud/1");
  await expect(page.getByTestId("hud-scale")).toHaveText("100 %");
  await page.getByTestId("hud-bigger").click();
  await expect(page.getByTestId("hud-scale")).toHaveText("110 %");
  await page.getByTestId("hud-format").selectOption("6");
  await expect(page.getByTestId("hud-format")).toHaveValue("6");
  await expect(page.getByTestId("hud-panel")).toHaveCount(2);
});

test("HUD overlay: right-click menu and double-click open the player sheet", async ({ page }) => {
  await page.goto("/#/hud/1");
  const panel = page.getByTestId("hud-panel").first();
  await panel.click({ button: "right" });
  await expect(page.getByTestId("hud-menu")).toBeVisible();
  await expect(page.getByTestId("hud-menu-hide")).toContainText("Masquer le HUD de cette table");
  await page.mouse.click(5, 5);
  await expect(page.getByTestId("hud-menu")).toHaveCount(0);
  await panel.dblclick();
  await expect(page).toHaveURL(/#\/m\/tracker\/opponents\?player=/);
});
