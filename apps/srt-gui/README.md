# Vesta desktop app

Tauri 2 desktop interface for the Vesta subtitle and flashcard toolchain. The frontend uses Svelte 5, TypeScript, Vite, and Tailwind CSS; native commands live in `src-tauri`.

```bash
pnpm install
pnpm check
pnpm test
pnpm tauri dev
```

Build the production frontend with `pnpm build`, or the complete desktop bundle with `pnpm tauri build`.
