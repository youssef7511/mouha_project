# Optional: export changes saved in your browser

The ZIP contains source and seed data. If you created real projects or entered other records in the hosted preview, those changes are in that browser's localStorage and are not automatically included here.

Use the same browser/profile where you made those changes. Open the PackFlow preview and its browser developer console. Run this snippet to download a JSON snapshot:

```javascript
(() => {
  const keys = [
    'packflow-projects',
    'packflow-clients',
    'packflow-stock',
    'packflow-finance-v1'
  ];
  const snapshot = {
    format: 'packflow-prototype-export',
    version: 1,
    exportedAt: new Date().toISOString(),
    sourceOrigin: location.origin,
    data: Object.fromEntries(keys.map(key => {
      const value = localStorage.getItem(key);
      return [key, value === null ? null : JSON.parse(value)];
    }))
  };
  const url = URL.createObjectURL(new Blob(
    [JSON.stringify(snapshot, null, 2)],
    { type: 'application/json' }
  ));
  const link = document.createElement('a');
  link.href = url;
  link.download = 'packflow-prototype-data.json';
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
})();
```

This reads the four PackFlow keys and downloads a file; it does not change your records. A `null` value means that key has no saved records. The current UI can still show unsaved seed defaults in that case, so this export is not automatically a complete database backup.

If you want the production app to import it, give this JSON to Codex alongside the project. Ask for a validated migration with an import preview, reference checks, duplicate detection, and inventory opening balances. Historical costs must not deduct stock again. Do not replace the production database by blindly loading this JSON.

Keep exported customer, payment, and salary data in a location you control.
