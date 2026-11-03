import { expect, test } from "@playwright/test";

test("calculator: two players, add a third, invalid range blocks the run", async ({ page }) => {
  await page.goto("/#/m/equity/calc");
  await page.getByTestId("run").click();
  await expect(page.getByTestId("result-row")).toHaveCount(2);
  await expect(page.getByTestId("result-method")).toContainText("Monte-Carlo");
  await expect(page.getByTestId("equity-curves")).toBeVisible();
  await page.getByTestId("add-player").click();
  await expect(page.getByTestId("player-row")).toHaveCount(3);
  await page.getByTestId("run").click();
  await expect(page.getByTestId("result-row")).toHaveCount(3);
  await page.getByTestId("player-input").nth(2).fill("AKx");
  await expect(page.getByTestId("run")).toBeDisabled();
  await page.getByTestId("player-input").nth(2).fill("AhAd");
  // AsKs (player 1) is blocked by a board with the As: nothing to deal
  await page.getByTestId("calc-board").fill("AsKd2c");
  await expect(page.getByTestId("player-row").first()).toContainText("0 combos");
  await expect(page.getByTestId("run")).toBeDisabled();
  await page.getByTestId("calc-board").fill("7c8d2h");
  await expect(page.getByTestId("run")).toBeEnabled();
});

test("range editor: click, drag, suits, text, top %", async ({ page }) => {
  await page.goto("/#/m/equity/calc");
  await page.getByTestId("player-edit").nth(1).click();
  const ed = page.getByTestId("range-editor");
  await ed.getByTestId("range-clear").click();
  await expect(ed.getByTestId("range-count")).toContainText("0 combos");
  await ed.locator('[data-name="AA"]').click();
  await expect(ed.getByTestId("range-count")).toContainText("6 combos");
  await expect(ed.getByTestId("range-text")).toHaveValue("AA");
  // drag AKs → AJs (3 suited cells)
  const a = await ed.locator('[data-name="AKs"]').boundingBox();
  const b = await ed.locator('[data-name="AJs"]').boundingBox();
  await page.mouse.move(a!.x + a!.width / 2, a!.y + a!.height / 2);
  await page.mouse.down();
  await page.mouse.move(b!.x + b!.width / 2, b!.y + b!.height / 2, { steps: 6 });
  await page.mouse.up();
  await expect(ed.getByTestId("range-text")).toHaveValue("AA,AJs+");
  await expect(ed.getByTestId("range-count")).toContainText("18 combos");
  // one suit of AKs off
  await ed.getByTestId("range-suit-mode").click();
  await ed.locator('[data-name="AKs"]').click();
  await expect(ed.getByTestId("range-combo")).toHaveCount(4);
  await ed.getByTestId("range-combo").first().click();
  await expect(ed.getByTestId("range-count")).toContainText("17 combos");
  // text
  await ed.getByTestId("range-text").fill("TT+, AKo:50%");
  await expect(ed.getByTestId("range-count")).toContainText("36 combos");
  await ed.getByTestId("range-text").fill("TT+, AKz");
  await expect(ed.getByTestId("range-error")).toContainText("AKz");
  // top %
  await ed.getByTestId("range-top").fill("10");
  await expect(ed.getByTestId("range-count")).toContainText("(10");
  await page.getByTestId("modal-done").click();
  await expect(page.getByTestId("player-input").nth(1)).not.toHaveValue("TT+, AKz");
});

test("range library: save and load", async ({ page }) => {
  await page.goto("/#/m/equity/calc");
  await page.getByTestId("player-edit").nth(1).click();
  const ed = page.getByTestId("range-editor");
  await ed.getByTestId("range-library").locator("summary").click();
  await ed.getByTestId("range-text").fill("QQ+,AKs");
  await ed.getByTestId("range-lib-name").fill("6max/100bb/BTN_3bet");
  await ed.getByTestId("range-lib-save").click();
  await expect(ed.getByTestId("range-lib-msg")).toContainText("enregistrée");
  await ed.getByTestId("range-text").fill("22");
  await ed.getByTestId("range-lib-pick").selectOption("exemples/BTN_open");
  await ed.getByTestId("range-lib-load").click();
  await expect(ed.getByTestId("range-text")).toHaveValue(/^22\+,A2s\+/);
  await ed.getByTestId("range-lib-pick").selectOption("6max/100bb/BTN_3bet");
  await ed.getByTestId("range-lib-load").click();
  await expect(ed.getByTestId("range-text")).toHaveValue("QQ+,AKs");
});

test("range analysis: breakdown, keep top pair, equity buckets", async ({ page }) => {
  await page.goto("/#/m/equity/analysis");
  const top = page.locator('[data-testid="breakdown-made"] [data-id="top_pair"]');
  await expect(top).toBeVisible();
  await top.getByTestId("breakdown-check").check();
  await page.getByTestId("keep-selection").click();
  await expect(page.getByTestId("a-range").getByTestId("range-text")).toHaveValue(/^A/);
  await expect(page.locator('[data-testid="breakdown-made"] [data-id="top_pair"]')).toContainText("100,0 %");
  await page.getByTestId("villain-input").fill("TT+");
  await page.getByTestId("run-buckets").click();
  await expect(page.getByTestId("vs-equity")).toBeVisible();
  await expect(page.getByTestId("equity-histogram")).toBeVisible();
});

test("flop textures", async ({ page }) => {
  await page.goto("/#/m/equity/textures");
  await page.getByTestId("t-suits").selectOption("monotone");
  await page.getByTestId("t-run").click();
  await expect(page.getByTestId("t-summary")).toContainText("flops");
  await expect(page.getByTestId("t-row")).toHaveCount(2);
});

test("replayer opens the spot in Equity Lab", async ({ page }) => {
  await page.goto("/#/m/tracker/hands");
  await page.getByTestId("hand-row").first().click();
  // no need to play the hand: the link always carries the board as played
  await page.getByTestId("to-equity").click();
  await expect(page).toHaveURL(/#\/m\/equity\/calc/);
  await expect(page.getByTestId("calc-board")).toHaveValue(/^([2-9TJQKA][cdhs]){5}$/);
  await expect(page.getByTestId("player-input").first()).toHaveValue(/^[2-9TJQKA][cdhs][2-9TJQKA][cdhs]$/);
});
