import { expect, test } from "@playwright/test";

test("overview: result first, stats with explanations, € toggle", async ({ page }) => {
  await page.goto("/#/m/tracker");
  await expect(page.getByTestId("hero-result")).toContainText("bb");
  await expect(page.getByTestId("stat-vpip")).toContainText("Combien de mains tu joues");
  await expect(page.getByTestId("stat-pfr")).toBeVisible();
  await expect(page.getByTestId("stat-three_bet")).toBeVisible();
  await page.getByTestId("toggle-euros").check();
  await expect(page.getByTestId("hero-result")).toContainText("€");
  await expect(page.getByTestId("overview-hero")).toContainText("ROI");
});

test("report by position and graph in bb / €", async ({ page }) => {
  await page.goto("/#/m/tracker/report");
  await expect(page.getByTestId("report-table").locator("tbody tr")).toHaveCount(6);
  await page.goto("/#/m/tracker/graph");
  await expect(page.getByTestId("graph-bb").locator("canvas")).toBeVisible();
  await page.getByTestId("toggle-euros").check();
  await expect(page.getByTestId("graph-tourney").locator("canvas")).toBeVisible();
});

test("a stat created in the editor shows up in the overview", async ({ page }) => {
  await page.goto("/#/m/tracker/stats");
  await page.getByTestId("new-stat").click();
  await page.getByTestId("stat-id").fill("vpip_btn");
  await page.getByTestId("stat-name").fill("VPIP BTN");
  await page.getByTestId("stat-formula").fill("cnt_vpip / cnt_hands");
  await page.getByTestId("stat-where").fill("position = 'BTN'");
  await expect(page.getByTestId("stat-preview")).toContainText("25,0 %");
  await page.getByTestId("stat-save").click();
  await expect(page.getByTestId("stat-list")).toContainText("VPIP BTN");
  await page.getByRole("link", { name: "Vue d'ensemble" }).click();
  await expect(page.getByTestId("stat-vpip_btn")).toContainText("VPIP BTN");
});

test("hand list opens the replayer, which plays to the result", async ({ page }) => {
  await page.goto("/#/m/tracker/hands");
  await page.getByTestId("hand-row").first().click();
  await expect(page).toHaveURL(/#\/m\/replayer\/1$/);
  await expect(page.getByTestId("seat")).toHaveCount(3);
  await expect(page.getByTestId("step")).toHaveText("0 / 16");
  await page.getByTestId("step-next").click();
  await expect(page.getByTestId("step")).toHaveText("1 / 16");
  await page.getByTestId("step-last").click();
  await expect(page.getByTestId("board").locator("[data-card]")).toHaveCount(5);
  await expect(page.getByTestId("action-log")).toContainText("Résultat");
});

test("replayer quick filters and previous / next hand", async ({ page }) => {
  await page.goto("/#/m/replayer");
  const rows = page.getByTestId("hand-row");
  await expect(rows.first()).toBeVisible();
  const all = await rows.count();
  expect(all).toBeGreaterThan(1);
  await page.getByTestId("quick-player").fill("zzz-nobody");
  await expect(rows).toHaveCount(0);
  await page.getByTestId("quick-player").fill("");
  await expect(rows).toHaveCount(all);
  await rows.first().click();
  await expect(page.getByTestId("prev-hand")).toBeDisabled();
  await page.getByTestId("next-hand").click();
  await expect(page).toHaveURL(/#\/m\/replayer\/2$/);
  await expect(page.getByTestId("prev-hand")).toBeEnabled();
});
