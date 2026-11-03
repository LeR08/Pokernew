import { expect, test } from "@playwright/test";

test("solve, view, navigate, lock, re-solve, library", async ({ page }) => {
  await page.goto("/#/m/solver/new");
  await expect(page.getByTestId("solver-estimate")).toContainText("nœuds");
  await page.getByTestId("solver-start").click();
  // the finished solve opens in the viewer
  await expect(page).toHaveURL(/#\/m\/solver\/viewer/, { timeout: 10_000 });
  const v = page.getByTestId("viewer-0");
  await expect(v.getByTestId("viewer-action")).toHaveCount(3);
  await v.getByTestId("viewer-grid").locator('[data-name="AA"]').click();
  await expect(v.getByTestId("viewer-combos")).toContainText("As");
  await v.getByTestId("viewer-breakdown").click();
  await expect(v.getByTestId("viewer-bd")).toBeVisible();
  // bet, then IP: fold / call / raise
  await v.getByTestId("viewer-action").nth(1).click();
  await expect(v.getByTestId("viewer-path")).toContainText("OOP");
  await expect(v.getByTestId("viewer-action")).toHaveCount(3);
  // lock IP's reply, then re-solve with it
  await v.getByTestId("viewer-lock").click();
  await page.getByTestId("modal-done").click();
  await expect(page.getByTestId("solver-resolve")).toBeVisible();
  await page.getByTestId("solver-resolve").click();
  await expect(page).toHaveURL(/#\/m\/solver\/new/);
  await expect(page.getByText("verrouillé")).toBeVisible();
  // call → chance node (turn card)
  await page.getByTestId("solver-tab-library").click();
  await expect(page.getByTestId("library-row")).toHaveCount(1);
  await page.getByTestId("library-open").click();
  await page.getByTestId("viewer-0").getByTestId("viewer-action").nth(1).click();
  await page.getByTestId("viewer-0").getByTestId("viewer-action").nth(1).click();
  await expect(page.getByTestId("viewer-cards").locator("button")).toHaveCount(49);
});

test("bad input is refused before solving", async ({ page }) => {
  await page.goto("/#/m/solver/new");
  await page.getByTestId("solver-board").fill("AsAs");
  await expect(page.getByTestId("solver-start")).toBeDisabled();
});

test("compare two solutions side by side", async ({ page }) => {
  await page.goto("/#/m/solver/new");
  await page.getByTestId("solver-start").click();
  await expect(page).toHaveURL(/#\/m\/solver\/viewer/, { timeout: 10_000 });
  await page.getByTestId("solver-tab-library").click();
  await page.getByTestId("library-compare").click();
  await expect(page).toHaveURL(/#\/m\/solver\/compare/);
  await expect(page.getByTestId("viewer-0").getByTestId("viewer-grid")).toBeVisible();
  await expect(page.getByTestId("viewer-1").getByTestId("viewer-grid")).toBeVisible();
});

test("batch over a set of flops, report by texture", async ({ page }) => {
  await page.goto("/#/m/solver/batch");
  await expect(page.getByTestId("batch-count")).toContainText("6 flops");
  await page.getByTestId("batch-max").fill("3");
  await expect(page.getByTestId("batch-count")).toContainText("3 flops");
  await page.getByTestId("batch-start").click();
  await expect(page.getByTestId("batch-row")).toHaveCount(3, { timeout: 10_000 });
  await expect(page.getByTestId("batch-report")).toContainText("texture");
});
