import fs from "node:fs/promises";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { chromium } from "playwright";

async function captureState(page, outPath, percent) {
  await page.evaluate((p) => {
    const workInput = document.getElementById("work-timer");
    const breakInput = document.getElementById("break-timer");
    if (workInput) workInput.value = "3";
    if (breakInput) breakInput.value = "5";

    const normalized = Math.max(0, Math.min(100, p));
    const appCircle = document.getElementById("app-circle");
    if (appCircle) {
      appCircle.style.setProperty("--fatigue-progress", `${normalized}`);
      appCircle.classList.toggle("has-progress", normalized > 0);
      appCircle.classList.add("monitoring");
    }

    const display = document.getElementById("fatigue-display");
    if (display) display.textContent = `${Math.round(p)}%`;

    document.body.style.background =
      "radial-gradient(circle at 32% 20%, #8ec5ff 0%, #2d5687 45%, #121826 100%)";
  }, percent);

  const widget = page.locator("#app-circle");
  const box = await widget.boundingBox();

  if (box) {
    const viewport = page.viewportSize() || { width: 900, height: 900 };
    const pad = 8;
    const x = Math.max(0, box.x - pad);
    const y = Math.max(0, box.y - pad);
    const width = Math.min(viewport.width - x, box.width + pad * 2);
    const height = Math.min(viewport.height - y, box.height + pad * 2);
    await page.screenshot({ path: outPath, clip: { x, y, width, height } });
  } else {
    await page.screenshot({ path: outPath, fullPage: true });
  }
}

async function main() {
  const projectRoot = process.cwd();
  const htmlPath = path.join(projectRoot, "src", "index.html");
  const outDir = path.join(projectRoot, "artifacts", "progress-states");
  const states = [0, 33, 66, 100];

  await fs.mkdir(outDir, { recursive: true });

  const browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 340, height: 340 },
    deviceScaleFactor: 2
  });
  const page = await context.newPage();

  await page.goto(pathToFileURL(htmlPath).toString());
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(250);

  for (const percent of states) {
    const outPath = path.join(outDir, `widget-${percent}.png`);
    await captureState(page, outPath, percent);
    console.log(`Saved screenshot: ${outPath}`);
  }

  await browser.close();
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
