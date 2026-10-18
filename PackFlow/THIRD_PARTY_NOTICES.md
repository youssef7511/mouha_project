# Third-party notices

PackFlow's foundation uses the packages below. Their license files remain authoritative and must be included in distributable release notices. The npm and Cargo lockfiles record the complete dependency graph; review transitive licenses before distribution.

| Component | License / source |
| --- | --- |
| Tauri, CLI/API and single-instance plugin | MIT OR Apache-2.0 · https://github.com/tauri-apps |
| React, Vite, Tailwind, TanStack Query, React Hook Form/resolvers, Zod, Vitest, Prettier | MIT · respective package LICENSE files |
| TypeScript, Playwright | Apache-2.0 · respective package LICENSE files |
| Lucide | ISC · https://github.com/lucide-icons/lucide |
| rusqlite | MIT · https://github.com/rusqlite/rusqlite |
| rust_xlsxwriter 0.100.0 | MIT OR Apache-2.0 · https://github.com/jmcnamara/rust_xlsxwriter |
| SQLite | Public domain · https://www.sqlite.org/copyright.html |
| DM Sans and Manrope font data | SIL Open Font License 1.1 · fontsource package font license files |
| Fontsource package code | MIT · https://github.com/fontsource/fontsource |

Bundled font resources are imported locally by the Vite build. No font source server is contacted at runtime. PackFlow's SVG app icon is a code-created project asset. Excel generation uses rust_xlsxwriter locally; it does not require Excel or a cloud service. PDF generation remains deferred.
