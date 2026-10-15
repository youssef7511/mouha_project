# Local Mac account — proposed flow

15 October 2026. Implementation now includes native setup/login/recovery/password change/logout, Argon2 password hashes, persistent attempt limits, native command access checks and macOS Keychain remembered sessions. Browser account tests use an explicit IPC fixture; actual Keychain and installer acceptance still require a Mac. Phone and cloud work remain deferred. The current behavior and recovery procedures are in [OPERATIONS.md](../docs/OPERATIONS.md).

## User experience

1. On first launch, show Create your account before allowing access to business data. Collect display name, username, password and password confirmation. Recommend direct setup rather than a shared factory password. Account setup must be atomic and possible only when no account exists.
2. Show an offline recovery code once during setup and ask the owner to keep it separately. A recovery flow must preserve business records and invalidate previous sessions. No email reset or internet dependency is assumed.
3. Subsequent launches show username, password and Remember me (unchecked initially). Without Remember me, quitting the app ends the session. With it, this Mac can reopen the workspace until logout or credential reset. Describe this plainly beside the checkbox.
4. Provide Log out and Change password. Logout immediately revokes the remembered session and clears cached business data from the UI. Password changes require the current password or recovery code and revoke old sessions.

## Implementation requirements

- Enforce authentication in native read/write commands, not only by hiding React screens. Permit only account status/setup/login/recovery before authentication. Derive the audit actor from the authenticated account rather than accepting an arbitrary frontend actor ID.
- Store a salted password hash using Argon2id; never store the plaintext password. Store only hashed recovery codes and remembered-session verifiers in the database. Use random credentials and bounded login/recovery attempts.
- Store the persistent session secret in the macOS Keychain, not browser localStorage. Keep ordinary sessions in native process memory. Handle Keychain cancellation/failure by requiring login, without bypassing authentication.
- Preserve the existing business database and historical actor attribution during migration. Do not reset existing records or include an initialized owner/session in a distributable source package.
- Keep production authentication enforced. Any browser test setup must use an isolated test database and cannot become a production bypass.
- Restore must invalidate sessions and define account recovery explicitly. A local login is an application access gate; it does not encrypt the existing SQLite database or its backups.

## Verification before release

Test first setup, repeat-setup rejection, existing-data migration, wrong passwords, rate limiting, direct unauthenticated command rejection, logout, reopen with/without Remember me, password change, recovery and stale-session rejection. Verify the real Keychain flow on a Mac, including cancelled access. Authentication does not replace the pending backup/restore drill and Mac installer/runtime verification.

## References

- [OWASP password storage guidance](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [Apple Keychain services](https://developer.apple.com/documentation/security/keychain-services)


### Web preview login — 15 October 2026

The local browser preview now asks for account setup/login and shows a Log out button above the workspace. Preview credentials use the same Rust password/recovery implementation, stored separately in `.dev-data/preview-account.sqlite`; existing business records stay intact. Protected preview reads/writes require a valid HttpOnly, SameSite=Strict cookie. Remember me on the browser uses a 30-day cookie; ordinary login uses a session cookie (browser session-restore settings may preserve it). Logout/password reset revoke the token. This does not change the Mac Keychain implementation. Backup/restore file dialogs remain desktop-only.

The automated business-flow fixture may skip authentication only in a debug preview process explicitly configured with `PACKFLOW_TEST_NO_AUTH=1` and `/test-data` as its disposable data directory. The normal `/data` preview does not accept this bypass. The separate `preview-auth.spec.ts` test exercises real login and API rejection with authentication enabled on a fresh isolated server.
