CREATE TABLE mobile_links (
 workspace_id TEXT NOT NULL, entity TEXT NOT NULL CHECK(entity IN('client','order','payment')),
 source_id TEXT NOT NULL, target_id TEXT NOT NULL, source_version INTEGER NOT NULL,
 source_hash TEXT NOT NULL, local_hash TEXT NOT NULL, source_json TEXT NOT NULL CHECK(json_valid(source_json)),
 PRIMARY KEY(workspace_id,entity,source_id)
) STRICT;
CREATE TABLE mobile_imports (
 id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, device_id TEXT NOT NULL, file_hash TEXT NOT NULL,
 imported_at TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES actors(id), safety_backup TEXT NOT NULL,
 summary_json TEXT NOT NULL CHECK(json_valid(summary_json)), source_history_json TEXT NOT NULL CHECK(json_valid(source_history_json))
) STRICT;
