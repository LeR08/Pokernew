import { expect, test } from "@playwright/test";

test("first-run wizard: language, detected Winamax account, profile, import", async ({ page }) => {
  await page.goto("/?firstrun#/");
  await expect(page).toHaveURL(/#\/welcome$/);
  await page.getByTestId("welcome-next").click(); // language: keep French
  await expect(page.getByTestId("found-accounts")).toContainText("Tantaculejen");
  await expect(page.getByTestId("found-accounts")).toContainText("AppData\\Roaming\\winamax");
  await page.getByTestId("welcome-next").click(); // saves rooms/winamax.toml -> import starts
  await page.getByRole("button", { name: "mtt" }).click();
  await page.getByTestId("welcome-next").click();
  await expect(page.getByTestId("welcome-hands")).toHaveText("288 mains importées");
  await page.getByTestId("welcome-finish").click();
  await expect(page.getByTestId("module-tile")).toHaveCount(10);
  await expect(page.getByTestId("profile-switcher")).toHaveValue("mtt");
  // The configured folder is shown in the rooms settings and on the import screen.
  await page.getByTestId("module-tile").filter({ hasText: "Import" }).click();
  await expect(page.getByTestId("watched-folders")).toContainText("Tantaculejen");
  await expect(page.getByTestId("stat-hands")).toHaveText(/288/);
});

test("wizard can be skipped", async ({ page }) => {
  await page.goto("/?firstrun#/");
  await page.getByTestId("welcome-skip").click();
  await expect(page.getByTestId("module-tile")).toHaveCount(10);
});

test("manual import, error log with copy-as-test-case, backups", async ({ page, context }) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.goto("/#/m/import");
  await expect(page.getByTestId("stat-hands")).toHaveText("0");
  page.once("dialog", (d) => void d.accept("C:\\hh\\bad"));
  await page.getByTestId("import-folder").click();
  await expect(page.getByTestId("import-toast")).toContainText("288 nouvelles mains");
  await expect(page.getByTestId("import-history")).toContainText("manuel");
  await expect(page.getByTestId("import-errors")).toContainText("Tantaculejen dances");
  await page.getByRole("button", { name: "Copier comme cas de test" }).click();
  const clip = await page.evaluate(() => navigator.clipboard.readText());
  expect(clip).toContain("# line 17: unknown action");
  await page.getByTestId("backup-now").click();
  await expect(page.getByTestId("backups").locator("li")).toHaveCount(1);
});

test("rooms tab edits the watched folders", async ({ page }) => {
  await page.goto("/#/settings/rooms");
  await expect(page.getByTestId("folder-list")).toBeVisible();
  page.once("dialog", (d) => void d.accept("D:\\Poker\\history"));
  await page.getByRole("button", { name: "Ajouter un dossier…" }).click();
  await page.getByTestId("form-save").click();
  await page.goto("/#/m/import");
  await expect(page.getByTestId("watched-folders")).toHaveCount(0); // fresh mock: reload resets it
});
