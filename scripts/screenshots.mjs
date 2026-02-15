import { chromium } from 'playwright';

const BASE = 'http://localhost:3141';
const OUT = 'screenshots';

const themes = ['Slate', 'Indigo', 'Orchid'];

const pages = [
  { name: 'dashboard', path: '/' },
  { name: 'sessions', path: '/sessions' },
  { name: 'analytics', path: '/analytics' },
  { name: 'projects', path: '/projects' },
  { name: 'search', path: '/search?q=auth' },
];

async function run() {
  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: 2,
  });

  for (let ti = 0; ti < themes.length; ti++) {
    const themeName = themes[ti].toLowerCase();
    const page = await context.newPage();

    // Navigate to dashboard first and set the theme
    await page.goto(BASE, { waitUntil: 'networkidle' });
    // Wait for app to render
    await page.waitForSelector('.nav-links', { timeout: 10000 });

    // Set theme via localStorage and reload
    await page.evaluate((idx) => {
      localStorage.setItem('blacklight-theme', String(idx));
    }, ti);
    await page.reload({ waitUntil: 'networkidle' });
    await page.waitForSelector('.nav-links', { timeout: 10000 });
    // Give theme a moment to apply
    await page.waitForTimeout(500);

    for (const { name, path } of pages) {
      await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' });
      await page.waitForTimeout(800);
      const filename = `${OUT}/${name}-${themeName}.png`;
      await page.screenshot({ path: filename, fullPage: false });
      console.log(`  captured ${filename}`);
    }

    await page.close();
  }

  // Also capture a session detail page (first theme only)
  {
    const page = await context.newPage();
    await page.goto(BASE, { waitUntil: 'networkidle' });
    await page.evaluate(() => {
      localStorage.setItem('blacklight-theme', '0');
    });
    await page.goto(`${BASE}/sessions`, { waitUntil: 'networkidle' });
    await page.waitForSelector('.nav-links', { timeout: 10000 });
    await page.waitForTimeout(500);

    // Click the first session link
    const firstSession = await page.$('a[href*="/sessions/"]');
    if (firstSession) {
      await firstSession.click();
      await page.waitForTimeout(1500);
      await page.screenshot({ path: `${OUT}/session-detail-slate.png`, fullPage: false });
      console.log(`  captured ${OUT}/session-detail-slate.png`);
    }
    await page.close();
  }

  await browser.close();
  console.log('Done!');
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});
