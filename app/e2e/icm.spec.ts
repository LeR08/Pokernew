import { expect, test } from "@playwright/test";

test("payout structure: enter, paste CSV, save, reuse in the ICM calculator", async ({ page }) => {
  await page.goto("/#/m/icm/payouts");
  await page.getByTestId("payouts-name").fill("Space KO test");
  await page.getByTestId("payouts-csv").locator("..").locator("summary").click();
  await page.getByTestId("payouts-csv").fill("place;gain\n1;60\n2;25\n3;15");
  await page.getByTestId("payouts-csv-apply").click();
  await expect(page.getByTestId("payouts-place")).toHaveCount(3);
  await expect(page.getByTestId("payouts-place").first()).toHaveValue("60");
  await page.getByTestId("payouts-save").click();
  await expect(page.getByTestId("payouts-msg")).toContainText("enregistrée");
  await expect(page.getByTestId("payouts-list")).toContainText("Space KO test");
  await page.getByTestId("icm-tab-calc").click();
  await page.getByTestId("calc-payouts-saved").selectOption({ label: "Space KO test" });
  await expect(page.getByTestId("calc-payouts")).toHaveValue("60, 25, 15");
  await expect(page.getByTestId("icm-total")).toContainText("100.00 €");
});

test("ICM calculator: equities add up to the prize pool, bubble factors", async ({ page }) => {
  await page.goto("/#/m/icm/calc");
  await expect(page.getByTestId("icm-row")).toHaveCount(3);
  await expect(page.getByTestId("icm-total")).toContainText("100.00 €");
  await page.getByTestId("icm-add").click();
  await expect(page.getByTestId("icm-row")).toHaveCount(4);
  await expect(page.getByTestId("icm-total")).toContainText("100.00 €");
  await expect(page.getByTestId("bubble-matrix")).toBeVisible();
});

test("push/fold: grid per decision, players, ICM model, lock a range", async ({ page }) => {
  await page.goto("/#/m/icm/pushfold");
  await expect(page.getByTestId("pf-solution")).toBeVisible();
  await expect(page.getByTestId("class-grid").locator('[data-name="AA"]')).toHaveAttribute("data-freq", "1.00");
  await page.getByTestId("pf-node").selectOption({ index: 1 });
  await expect(page.getByTestId("pf-share")).toBeVisible();
  await page.getByTestId("pf-players").selectOption("6");
  await expect(page.getByTestId("pf-stack")).toHaveCount(6);
  await expect(page.getByTestId("pf-node").locator("option")).toHaveCount(5 + 15);
  await page.getByTestId("pf-model").selectOption("icm");
  await expect(page.getByTestId("pf-payouts")).toBeVisible();
  await page.getByTestId("pf-node").selectOption({ index: 1 });
  await page.getByTestId("pf-lock").click();
  await page.getByTestId("lock-editor").getByTestId("range-text").fill("QQ+,AK");
  await page.getByTestId("modal-done").click();
  await expect(page.getByTestId("pf-locks")).toContainText("QQ+,AK");
});

test("hand review opens from the replayer, batch review from the tournament list", async ({ page }) => {
  await page.goto("/#/m/tracker/hands");
  await page.getByTestId("hand-row").first().click();
  await page.getByTestId("to-icm").click();
  await expect(page).toHaveURL(/#\/m\/icm\/review\?hand=1/);
  await expect(page.getByTestId("review-verdict")).toContainText("meilleure option");
  await expect(page.getByTestId("loss-chips")).toBeVisible();
  await page.goto("/#/m/tracker/tournaments");
  await page.getByTestId("tournament-icm").first().click();
  await expect(page.getByTestId("review-batch")).toBeVisible();
  await expect(page.getByTestId("batch-row")).toHaveCount(1);
});

test("quiz: answer a spot and get scored", async ({ page }) => {
  await page.goto("/#/m/icm/quiz");
  await page.getByTestId("quiz-start").click();
  await page.getByTestId("quiz-fold").click();
  await expect(page.getByTestId("quiz-result")).toBeVisible();
  await expect(page.getByTestId("quiz-score")).toContainText("/ 1");
});
