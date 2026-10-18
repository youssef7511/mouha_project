# PackFlow phone-to-Mac Excel transfer, format 1

Shared Rust crate used by PackFlow Mobile 0.2 and PackFlow desktop 0.4. Both projects must remain alongside this directory when building from source. Installed applications have no dependency on each other or on these source folders.

On the phone, open Settings → Transfer to Mac, select Changed from/through and export. Share the resulting `.xlsx` from Files/Downloads. On the Mac, open Clients, Orders, Payments or Settings → Import from phone. Choose the workbook, match new phone clients to existing Mac clients or explicitly create new contacts, preview again, review the rows and import ready records.

The workbook is a strict, versioned transfer document. Do not edit its cells, columns, worksheet names or formulas; export a fresh file from the app if the source data changes. Existing reporting Excel exports and database backups are different formats.

## Workbook layout

All cells are text to preserve IDs, leading-zero phones, exact integer fils, scaled unit quantities, dates and unknown/null amounts. Empty optional amounts mean unknown/not applicable; `0` means a deliberate zero. No Excel formulas are required: the Mac recalculates quotes using its own checked integer rules.

| Worksheet | Content |
| --- | --- |
| Transfer information | Format/version, KWD, workspace/device IDs, export timestamp, dates and content checksum |
| Clients | Stable source IDs, version, contact fields, notes and archive metadata |
| Orders | Source mobile number, client reference, version, dates, notes and draft/confirmed metadata |
| Box types | Source box IDs, order relationship, position, description, dimensions and ordered quantity |
| Stages | Selected stage IDs/names, order and position, pricing mode and fixed/unit inputs |
| Stage box prices | Stage-to-box relationship and exact unit selling price |
| Payments | Receipts/corrections, original receipt relationship, amount, date, method and references |
| Quote history | Historical phone quote documents, version, reason, time and device provenance |

The fixed column definitions are in `src/lib.rs`. Limits: 8 MiB compressed file, 32 MiB expanded workbook, 100,000 total data rows, 5,000 orders and 20,000 payments per export. Individual Excel text-cell limits also apply; the exporter reports overflow rather than truncating a document. XML worksheet references are bounded before spreadsheet range allocation. Edited checksums, formulas, invalid types/relationships, unknown sheets/columns and malformed prices are rejected.

## Date selection and dependencies

The selected period uses the Kuwait calendar date of creation/update on the phone. It selects changed clients and orders, including orders with new receipts/corrections. A selected order brings its client, every current box/stage price, all its receipts and corrections, and its source quote history, even if those related records are older than the range. The workbook is a complete related snapshot, not a period-only balance report. A backdated payment entered today is selected by today's creation date. Creation and audit history select records even if they were edited again after the selected period. Exported values are the latest saved snapshot at export time, not an as-of-period reconstruction.

## Import semantics

- Permanent mapping keys are `(source workspace UUID, entity kind, source UUID)`. Mac IDs and official PF numbers are assigned on first import; the MOB number remains visible as provenance. Names alone never identify a receipt/order.
- The client matching step prevents silently duplicating an existing contact. Linking an existing Mac client preserves its contact fields. Newly imported contacts can accept later source updates only while their Mac details remain unchanged.
- Repeated overlapping exports skip identical records. Changed data with the same/older version is a conflict. Imported receipts cannot be edited through another workbook.
- Unchanged Mac orders accept newer phone quotes when the box/stage identities and sequence are unchanged. The Mac appends its own quote revision. If the Mac order/quote/production/work/cost/payment state has changed, or the phone changes its box/stage structure, the order is flagged for manual review and is not overwritten. Dependent new payments on a conflicted order are blocked. This V1 importer does not provide an automatic conflict merge/resolution editor.
- New orders enter Mac production with stages not started. Phone draft/confirmed status and archive flags remain source metadata; they do not close, cancel or archive Mac production. Missing phone production due dates stay visibly unset. Customer payment deadlines remain separate.
- Receipts are imported once. Phone corrections become full Mac receipt reversals/refund allocations, retaining original history. If the original was already refunded/corrected on Mac, another reversal is a conflict. Actual Mac supplier expenses, stock and payroll are not inferred from phone customer pricing.
- Source quote history, original client notes and source documents remain in the import/provenance tables. Phone historical documents are not fabricated as historical Mac production records. Current box/stage pricing is mapped to the Mac's canonical quote tables.

Preview parses the file, validates all quotes and runs a complete import on a disposable copy of the current Mac database. It writes no live records. A preview token binds the unchanged file, matching decisions and complete current workspace state. Apply rechecks/rebuilds that plan, saves a `.pfbackup` safety copy, then atomically copies the validated candidate into the live SQLite database. Failed validation or stale preview cannot partially modify business records. Use the existing backup restore workflow to undo an applied batch; restoring the safety backup rolls back the whole workspace to that moment.

This is one-way transfer, not synchronization or a backup. Mac changes and Mac-only receipts do not return to the phone; its independent balance is not the full Mac balance. Enter a payment on one device only—IDs prevent importing the same phone event twice, but cannot identify an independently typed duplicate Mac receipt. Transfer files contain business data; the checksum detects changes/corruption and is not encryption or authentication.

Cross-app integration tests live in `PackFlow/crates/packflow-core/tests/phone_transfer.rs` and produce real mobile exports for the Mac browser test when `PACKFLOW_TRANSFER_TEST_OUTPUT` is set. No workbook or business database is checked into Git.
