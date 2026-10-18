# iPhone acceptance before daily use

Automated browser and Rust tests are development evidence. Complete these checks on the intended signed iPhone build; do not treat simulator evidence as a physical-device pass.

- [ ] Install and launch without Rust, Docker, a server or the Mac running. Confirm no clients/orders/receipts appear on first launch, only four default stage definitions.
- [ ] Turn on airplane mode. Create a client, an order with two differently sized box types, quantities and dimensions. Enter sheet notes. Set each stage pricing mode and verify the quote in three-decimal KWD.
- [ ] Record a deposit at order creation. Close/reopen the application while offline; verify all details, receipt and balance remain.
- [ ] Receive another payment, test an overpayment/credit, correct a mistaken receipt with a reason, and enter the replacement. Confirm original and correction remain visible and balances are correct.
- [ ] Revise a priced order and verify prior quotes remain in history. Archive it and confirm history/debt remains searchable.
- [ ] Rename, add, archive and reorder stage definitions. Check new orders reflect the definitions while existing orders retain their saved names and prices.
- [ ] Exercise the smallest supported phone screen, portrait/landscape, keyboard, date/number input, scrolling, fixed footers, safe areas and unsaved-change confirmation.
- [ ] Save a backup. Locate it in Files, copy it outside the phone, make another record, restore the earlier backup, then restore the safety backup to undo. Reject a damaged backup without losing current records.
- [ ] Update the signed application using the same identifier/team and verify existing local data survives. Do not uninstall to test an update; uninstalling removes application data.
- [ ] Check device screen-lock protection, confirm the standalone balance disclaimer, and agree on external backup responsibility. This version has no app-specific login or automatic Mac transfer.

For future transfer, keep independent phone and Mac entry procedures clear. Entering the same receipt on both creates two separate records today; neither app can detect that across devices yet.
