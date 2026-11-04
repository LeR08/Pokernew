import { expect, test } from "@playwright/test";

test("postflop drill from a cached solution: answer, grade, full hand, history", async ({ page }) => {
  await page.goto("/#/m/trainer/postflop");
  await expect(page.getByTestId("drill-empty")).toBeVisible();
  // make a solution in the solver (same page session = same mock backend)
  await page.goto("/#/m/solver/new");
  await page.getByTestId("solver-start").click();
  await expect(page).toHaveURL(/#\/m\/solver\/viewer/, { timeout: 10_000 });
  await page.goto("/#/m/trainer/postflop");
  await page.getByTestId("drill-start").click();
  await expect(page.getByTestId("drill-question")).toBeVisible();
  await expect(page.getByTestId("drill-action")).toHaveCount(3);
  await page.getByTestId("drill-action").first().click();
  await expect(page.getByTestId("drill-grade")).toContainText("perte");
  await expect(page.getByTestId("drill-stats")).toContainText("1 question");
  // full hand: the next decision of the same hand
  await page.getByTestId("drill-mode").selectOption("hand");
  await page.getByTestId("drill-next").click();
  await page.getByTestId("drill-action").nth(1).click();
  await expect(page.getByTestId("drill-grade")).toContainText("Meilleur coup");
  await page.getByTestId("drill-next").click();
  await expect(page.getByTestId("drill-question")).toBeVisible();
  await page.getByTestId("trainer-tab-history").click();
  await expect(page.getByTestId("hist-counts")).toContainText("2 réponse");
});

test("preflop drill from the default charts", async ({ page }) => {
  await page.goto("/#/m/trainer/preflop");
  await expect(page.getByTestId("pre-folder")).toHaveValue("cash 6max 100bb");
  await page.getByTestId("pre-start").click();
  await expect(page.getByTestId("pre-question")).toBeVisible();
  await page.getByTestId("pre-action").first().click();
  await expect(page.getByTestId("pre-answer")).toBeVisible();
  await expect(page.getByTestId("pre-stats")).toContainText("sur 1");
});

test("hand analysis from the tracker: report by hand and by spot", async ({ page }) => {
  await page.goto("/#/m/tracker/hands");
  await page.getByTestId("hands-analyse").click();
  await expect(page).toHaveURL(/#\/m\/trainer\/analysis\?hands=/);
  await page.getByTestId("ana-start").click();
  await expect(page.getByTestId("ana-report")).toBeVisible({ timeout: 10_000 });
  await expect(page.getByTestId("ana-leaks")).toContainText("turn · PFR · IP");
  await expect(page.getByTestId("ana-row").first()).toContainText("bb");
});

test("push/fold drills are in the trainer too", async ({ page }) => {
  await page.goto("/#/m/trainer/pushfold");
  await page.getByTestId("quiz-start").click();
  await page.getByTestId("quiz-fold").click();
  await expect(page.getByTestId("quiz-result")).toBeVisible();
});
