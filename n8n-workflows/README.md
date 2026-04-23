# n8n Workflow Import Instructions

## 1. Release Tweet Update (`release-tweet-updated.json`)

**What changed vs. current workflow:**
- Tweet limit raised from 160 to **200 characters**
- Extracts first meaningful line from release notes as tweet highlight
- Progressive fallback: tries full tweet (highlight + link + tags) → shorter versions
- Discord install block updated to include `lean-ctx update` as first option

**How to update:**
1. Open n8n: `http://n8n-ggs44kcog0so4s0w4sc88gwg.n8n.185-142-213-170.sslip.io`
2. Login: `yves@pounce.ch` / `LeanCtx2026!`
3. Open workflow "LeanCTX Release → Discord + Twitter"
4. Click on the **"Build Payloads"** node
5. Replace the JavaScript code with the code from the "Build Payloads" node in `release-tweet-updated.json`
6. Save & activate

**Alternatively (full import):**
1. Deactivate the current "LeanCTX Release → Discord + Twitter" workflow
2. Import `release-tweet-updated.json` via Menu → Import from File
3. Activate the new workflow
4. Delete the old one

## 2. Daily Tips Workflow (`daily-tweets.json`)

**What it does:**
- Posts **2 tweets per day** (09:30 UTC + 17:00 UTC)
- Rotates through **30 unique tips** (60 total morning/evening slots)
- Categories: features, pro tips, stats, trust/privacy, call-to-action
- Each tip is under 200 characters where possible
- Uses day-of-year for deterministic rotation (no repeats within a month)

**How to import:**
1. Open n8n UI
2. Menu → Import from File → select `daily-tweets.json`
3. **Activate** the workflow
4. It will start posting at the next scheduled time

**Content categories in rotation:**
- `feature` — Highlights a specific lean-ctx capability
- `tip` — "Pro tip:" practical usage advice
- `stats` — Token savings numbers, benchmark data
- `trust` — Privacy, open source, zero telemetry
- `cta` — Call to action with install command

**Cost estimate (Pay Per Use):**
- 2 tweets/day × 30 days = ~60 tweets/month
- Plus ~4-8 release tweets/month
- Total: ~70 tweets/month (well within $25 credit budget)
