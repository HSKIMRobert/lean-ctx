# **FAQ — Dashboard & Analytics**

---

**Q: How do I see my savings?**
```bash
lean-ctx gain              # terminal dashboard
lean-ctx gain --live       # real-time mode
lean-ctx gain --web        # opens web dashboard at localhost:3333
```

**Q: Dashboard shows 0% / no results!**
- Make sure your AI tool is actually using lean-ctx tools (check `lean-ctx doctor`)
- Shell hook savings and MCP savings are tracked separately
- Run a few AI-assisted coding tasks first, then check again
- Fixed display issues in v3.2.6 — update: `lean-ctx update`

**Q: "Dashboard indicates update available" but the version doesn't exist yet?**
This was a bug in v3.2.4 where the update check compared against an unreleased version. Fixed in v3.2.5+.

**Q: What is the Runtime Control Plane panel?**
The web dashboard (`lean-ctx gain --web`) includes a **Runtime Control Plane** panel that shows:

- **IDE indicator** — which IDE/agent is connected and its MCP capability tier (1–4)
- **Pressure gauge** — real-time context pressure with budget utilization percentage
- **Bounce stats** — number of bounces detected, tokens wasted, and learned patterns
- **Dynamic tool categories** — which of the 6 tool categories are currently loaded, with per-category call counts

This panel gives you a live view of how lean-ctx is adapting to your IDE and optimizing context in real time.
