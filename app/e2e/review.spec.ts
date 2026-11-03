import { expect, test } from "@playwright/test";

test("session review lists hands to review and good plays, linked to the replayer", async ({ page }) => {
  await page.goto("/#/m/tracker/sessions");
  await expect(page.getByTestId("session-list").locator("button")).toHaveCount(2);
  const review = page.getByTestId("session-review");
  await expect(review).toContainText("2 mains à revoir");
  await expect(review).toContainText("2 bons coups");
  await expect(page.getByTestId("finding-river_call_lost")).toContainText("Call river perdant (18,5 bb)");
  await expect(page.getByTestId("finding-steal_won")).toContainText("Vol de blindes réussi");
  await page.getByTestId("finding-steal_won").getByRole("link", { name: "Rejouer" }).click();
  await expect(page).toHaveURL(/#\/m\/replayer\/3$/);
});

test("leak finder shows leaks first with advice", async ({ page }) => {
  await page.goto("/#/m/tracker/leaks");
  const first = page.getByTestId("leaks").locator("> *").first();
  await expect(first).toContainText("Fuite");
  await expect(first).toContainText("défends plus souvent");
});

test("13x13 grid has 169 cells", async ({ page }) => {
  await page.goto("/#/m/tracker/grid");
  await expect(page.getByTestId("hand-grid").locator("[data-class]")).toHaveCount(169);
  await expect(page.getByTestId("hand-grid").locator('[data-class="AKs"]')).toBeVisible();
  await page.getByTestId("grid-metric").selectOption("hands");
});

test("bankroll: add a deposit, the balance follows", async ({ page }) => {
  await page.goto("/#/m/bankroll");
  await page.getByTestId("tx-amount").fill("50");
  await page.getByTestId("tx-add").click();
  await expect(page.getByTestId("tx-list")).toContainText("Dépôt");
  await expect(page.getByTestId("bankroll-balance")).toContainText("50,03");
});

test("notes on a player from the replayer", async ({ page }) => {
  await page.goto("/#/m/replayer/1");
  await page.getByTestId("seat").filter({ hasText: "Player10" }).click();
  await page.getByTestId("note-text").fill("relance large en SB");
  await page.getByTestId("note-add").click();
  await expect(page.getByTestId("notes-list")).toContainText("relance large en SB");
});

test("report CSV export", async ({ page }) => {
  await page.goto("/#/m/tracker/report");
  await page.getByTestId("export-report").click();
  await expect(page.getByText("Exporté :")).toBeVisible();
});
