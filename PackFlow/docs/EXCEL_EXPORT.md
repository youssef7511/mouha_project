# Excel exports (0.3.1)

Use **Export Excel** in the top bar of any page. The Mac app opens a native save dialog; the development browser downloads the same `.xlsx` workbook. Cancelling saves nothing. Choose a new filename when a file already exists; exports never overwrite existing files. Excel or Numbers is not required to generate a workbook.

| Page | Worksheets and scope |
| --- | --- |
| Dashboard | Current active orders and portfolio totals, monthly daily activity, cost categories and stage progress. |
| Orders | All orders matching the search, including rows beyond pagination; box types, stage customer pricing, per-box stage prices, stage progress/work, costs, customer payments and outside services. |
| Clients | Complete directory including marked archived clients, plus the app's canonical payment statistics. |
| Workers | Complete worker directory, salary inputs/results for the month selected in Monthly salaries, monthly divisors and nonarchived salary totals. The export API also supports all months by omitting `month`. |
| Costs | All order expenses and their reversals, outside services, and separate workspace expenses and their dated corrections. |
| Payments & balances | All active orders matching the balance search, their lifetime balances, customer payments/refunds and outside services. |
| Reports | Current active order performance matching its search; portfolio totals and stage progress; daily activity, cost categories, stage expenses, payments, outside services and separate costs for the selected date range. |
| Settings | Stage templates, including archived ones, and business settings. No credentials. |
| Stock | An empty stock worksheet. Inventory remains deferred; no records are invented. |

Every workbook includes **Export information** describing its scope and timestamp. Export uses one consistent read transaction through the database worker, not the first UI page or the old 200-record directory limit. Report search affects only current order performance: its financial charts and dated ledger sheets cover all historical orders, including archived/cancelled orders, exactly as the report queries do. Current portfolio balances are lifetime snapshots and are not recalculated as historical balances for the selected event dates.

KWD amounts are numeric with three decimal places. Null/unpriced amounts are blank; recorded zero stays zero. Scaled quantities/hours/days are converted back to their entered decimal units. IDs, phone numbers, descriptions and ISO dates are text. User text is never evaluated as a formula or automatically made a hyperlink. Values exceeding Excel's 15 significant digits are written as exact decimal text to avoid losing precision.

Original ledger records remain visible. Expense reversals, refunds and separate-cost reversals are signed negative amounts; separate-cost reversals use their correction date. Outside-service supplier costs are already present in stage expenses; their service worksheet is explanatory, not another cost to add. Voided services retain their original amounts with a void flag/date/reason. Work reversals retain their recorded quantities and a reversal kind; those rows are historical events, not precomputed net production quantities. Salary worksheets show calculated payroll, not salary payments or costs allocated to orders.

Excel files are for analysis/sharing. They do not include account passwords, recovery codes or remembered-session tokens, and cannot restore the workspace. Continue using `.pfbackup` for backup/restore. Export makes no database/schema changes. Files exceeding Excel's worksheet or cell-text limits fail visibly instead of truncating silently.

The Mac save dialog, cancel behaviour, exporting to a normal folder and choosing an unwritable destination should be included in the real-Mac acceptance drill. The native export command has the same sign-in requirement as other protected commands.
