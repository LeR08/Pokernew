import { expect, test } from "@playwright/test";

test("tiles and sidebar come from the module manifests", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByTestId("module-tile")).toHaveCount(10);
  await expect(page.getByTestId("sidebar-modules").getByRole("link")).toHaveCount(10);
  await page.getByTestId("module-tile").filter({ hasText: "Ranges & solveur préflop" }).click();
  await expect(page).toHaveURL(/#\/m\/preflop$/);
  await expect(page.getByTestId("coming-soon")).toContainText("phase 7");
});

test("settings form changes theme, language and module visibility live", async ({ page }) => {
  await page.goto("/#/settings/general");
  await page.getByLabel("Thème").selectOption("light");
  await page.getByLabel("Langue").selectOption("en");
  await page.getByRole("switch", { name: "Tracker" }).click();
  await page.getByTestId("form-save").click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
  await expect(page.getByRole("heading", { name: "Settings centre" })).toBeVisible();
  await expect(page.getByTestId("sidebar-modules").getByRole("link")).toHaveCount(9);
});

test("raw editor validates against the schema before saving", async ({ page }) => {
  await page.goto("/#/settings/themes");
  await page.getByRole("tab", { name: "Texte brut" }).click();
  const editor = page.locator(".cm-content");
  await expect(editor).toContainText("base = \"dark\"");
  await editor.click();
  await page.keyboard.press("ControlOrMeta+End");
  await page.keyboard.type('\nextra = 1');
  await expect(page.getByTestId("raw-issues")).toBeVisible();
  await expect(page.getByTestId("raw-save")).toBeDisabled();
  await page.keyboard.press("ControlOrMeta+z");
  await expect(page.getByTestId("raw-status")).toBeVisible();
});

test("profile switch and hotkeys", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("profile-switcher").selectOption("mtt");
  await expect(page.getByTestId("profile-switcher")).toHaveValue("mtt");
  await page.keyboard.press("Control+,");
  await expect(page).toHaveURL(/#\/settings$/);
  await page.keyboard.press("Control+h");
  await expect(page).toHaveURL(/#\/$/);
});

test("drag and drop reorders tiles and saves the order", async ({ page }) => {
  await page.goto("/");
  const tiles = page.getByTestId("module-tile");
  await expect(tiles.first()).toHaveAttribute("data-module", "import");
  const src = await tiles.nth(1).boundingBox();
  const dst = await tiles.nth(0).boundingBox();
  await page.mouse.move(src!.x + 40, src!.y + 40);
  await page.mouse.down();
  await page.mouse.move(src!.x + 20, src!.y + 30, { steps: 5 });
  await page.mouse.move(dst!.x + 30, dst!.y + 40, { steps: 15 });
  await page.mouse.up();
  await expect(tiles.first()).toHaveAttribute("data-module", "tracker");
  await expect(page.getByTestId("sidebar-modules").getByRole("link").first()).toHaveAttribute("data-module", "tracker");
});
