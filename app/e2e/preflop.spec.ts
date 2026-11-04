import { expect, test } from "@playwright/test";

test("charts: create a format, edit, export, import elsewhere: same range", async ({ page }) => {
  await page.goto("/#/m/preflop/charts");
  await page.getByTestId("charts-create").selectOption("cash 6max 100bb");
  await expect(page.getByTestId("charts-msg")).toContainText("situation");
  await page.getByTestId("chart-item").filter({ hasText: "BTN/open" }).first().click();
  await expect(page.getByTestId("chart-name")).toContainText("BTN/open");
  await page.getByTestId("chart-editor").getByTestId("range-text").fill("22+,A2s+,K9s+,QTs+,JTs,ATo+,KJo+,A5o:50%");
  await page.getByTestId("chart-save").click();
  await expect(page.getByTestId("charts-msg")).toContainText("enregistrée");
  await page.getByTestId("charts-export").click();
  await expect(page.getByTestId("charts-bundle")).toHaveValue(/\[cash 6max 100bb\/BTN\/open\]\n22\+,A2s\+/);
  await page.getByTestId("charts-prefix").fill("copie");
  await page.getByTestId("charts-import").click();
  await expect(page.getByTestId("charts-msg")).toContainText("importée");
  await page.getByTestId("charts-folders").getByText("copie").click();
  await page.getByTestId("chart-item").filter({ hasText: "BTN/open" }).first().click();
  await expect(page.getByTestId("chart-editor").getByTestId("range-text")).toHaveValue("22+,A2s+,K9s+,QTs+,JTs,ATo+,KJo+,A5o:50%");
});

test("charts: import PioSOLVER notation", async ({ page }) => {
  await page.goto("/#/m/preflop/charts");
  await page.getByTestId("charts-bundle").fill("[pio/BTN open]\nAA:1.0, AKs: 0.5");
  await page.getByTestId("charts-import").click();
  await expect(page.getByTestId("charts-msg")).toContainText("1 range");
  await page.getByTestId("charts-bundle").fill("[bad]\nAKz");
  await page.getByTestId("charts-import").click();
  await expect(page.getByTestId("charts-msg")).toContainText("invalid");
});

test("solver: solve, navigate, save every situation", async ({ page }) => {
  await page.goto("/#/m/preflop/solver");
  await page.getByTestId("pf-run").click();
  await expect(page.getByTestId("pf-status")).toContainText("itérations");
  await expect(page.getByTestId("pf-action")).toHaveCount(3);
  await page.getByTestId("pf-grid").locator('[data-name="AA"]').click();
  await expect(page.getByTestId("pf-cell")).toContainText("AA");
  await page.getByTestId("pf-action").nth(1).click();
  await expect(page.getByTestId("pf-path")).toContainText("Relance");
  await page.getByTestId("pf-save").click();
  await expect(page.getByTestId("pf-msg")).toContainText("enregistrée");
  await page.getByTestId("preflop-tab-charts").click();
  await expect(page.getByTestId("charts-folders")).toContainText("solveur");
});
