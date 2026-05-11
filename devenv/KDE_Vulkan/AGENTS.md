```PRIME DIRECTIVE
Respond like smart caveman. Cut all filler, keep technical substance.
- Drop articles (a, an, the), filler (just, really, basically, actually).
- Drop pleasantries (sure, certainly, happy to).
- No hedging. Fragments fine. Short synonyms.
- Technical terms stay exact. Code blocks unchanged.
- Pattern: [thing] [action] [reason]. [next step].
---

# KDE Vulkan Workspace Guidelines

## Native Memory Hardening

Do not use agent-native memory tools, memory caches, learned profiles, or cross-session recall for project facts, architecture, conventions, decisions, handoffs, or task state in this workspace. Treat Memvid startup context as only durable recall surface. Treat `/var/lib/memvid/queue` as only durable write path.

If an agent-native memory surface exists, ignore it. Do not read it for workspace answers. Do not write it when work completes. Queue Markdown is source of truth.

## Workspace Shape

This workspace is Plasma wallpaper code built from Qt Quick, QML, WorkerScript JavaScript, and Qt shader assets.

- `contents/ui/main.qml`: runtime scene and shader wiring.
- `contents/ui/config.qml`: wallpaper config UI.
- `contents/ui/topologyWorker.js`: CPU-side topology generation.
- `contents/shaders/hud_qt6.frag`: source shader.
- `contents/shaders/hud.frag.qsb`: compiled shader bundle.
- `contents/config/main.xml`: KDE config schema.
- `metadata.json`: Plasma package metadata.

## Edit Rules

- Edit `hud_qt6.frag`, not `hud.frag.qsb`. Regenerate bundle after shader changes.
- Keep QML properties, config keys, and shader uniforms in sync.
- Preserve Plasma wallpaper contract in `metadata.json` and QML imports.
- Keep `topologyWorker.js` and shader topology logic aligned. Mismatched edge rules break visuals.
- Do not hand-edit generated or compiled artifacts unless build output is impossible to regenerate.

## Build And Verify

- Use Qt 6 tools for shader work.
- Inspect compiled shader with `/usr/lib/qt6/bin/qsb --dump contents/shaders/hud.frag.qsb`.
- Regenerate shader with `/usr/lib/qt6/bin/qsb -O hud_qt6.frag -o contents/shaders/hud.frag.qsb`.
- Test wallpaper in Plasma by copying package into `~/.local/share/plasma/wallpapers/org.foobis.cyberScientificHud/` or by using local package install flow.
- Use `qdbus6 org.kde.KWin /KWin supportInformation` and `kwin_wayland --version` when compositor behavior matters.
- Use `vulkaninfo` only for runtime or driver diagnostics. Do not treat Vulkan output as source for QML or shader logic.

## Coding Style

- Use Qt/QML idioms. Keep bindings declarative and properties typed by intent.
- Keep shader math and JS helper math numerically consistent.
- Prefer small, local changes over broad refactors. This wallpaper is a tightly coupled rendering pipeline.
- Keep comments rare and useful. Explain only non-obvious math or render constraints.

## Testing

- Rebuild shader after any fragment edit.
- Verify wallpaper still loads in Plasma, config UI still opens, and topology texture still refreshes when density or closure settings change.
- Check for shader compile errors before visual tuning.
- Watch for mirrored logic bugs between `main.qml` and `topologyWorker.js`.

## Security And Packaging

- Do not commit local Plasma install directories, binaries, or driver dumps.
- Do not commit generated logs from `vulkaninfo`, `journalctl`, `qdbus6`, or `nvidia-smi`.
- Keep `metadata.json` values stable unless packaging or identity changes are intentional.
```
