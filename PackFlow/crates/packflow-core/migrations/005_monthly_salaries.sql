CREATE TABLE salary_months (
 month TEXT PRIMARY KEY CHECK(length(month)=7 AND month GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]'),
 divisor INTEGER NOT NULL CHECK(divisor BETWEEN 1 AND 31),
 version INTEGER NOT NULL DEFAULT 1 CHECK(version>0), created_at TEXT NOT NULL, updated_at TEXT NOT NULL
) STRICT;
CREATE TABLE monthly_salaries (
 id TEXT PRIMARY KEY, month TEXT NOT NULL REFERENCES salary_months(month), worker_id TEXT NOT NULL REFERENCES workers(id),
 worker_name_snapshot TEXT NOT NULL, basic_fils INTEGER NOT NULL CHECK(basic_fils BETWEEN 0 AND 9000000000000),
 normal_days_scaled INTEGER NOT NULL CHECK(normal_days_scaled BETWEEN 0 AND 31000),
 hours_per_day_scaled INTEGER NOT NULL CHECK(hours_per_day_scaled BETWEEN 1 AND 24000),
 fridays INTEGER NOT NULL CHECK(fridays BETWEEN 0 AND 31),
 overtime_hours_scaled INTEGER NOT NULL CHECK(overtime_hours_scaled BETWEEN 0 AND 744000),
 normal_fils INTEGER NOT NULL CHECK(normal_fils BETWEEN 0 AND 9000000000000),
 friday_fils INTEGER NOT NULL CHECK(friday_fils BETWEEN 0 AND 9000000000000),
 overtime_fils INTEGER NOT NULL CHECK(overtime_fils BETWEEN 0 AND 9000000000000),
 total_fils INTEGER NOT NULL CHECK(total_fils BETWEEN 0 AND 9000000000000 AND total_fils=normal_fils+friday_fils+overtime_fils),
 notes TEXT NOT NULL DEFAULT '', archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0,1)),
 version INTEGER NOT NULL DEFAULT 1 CHECK(version>0), created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
 UNIQUE(month,worker_id)
) STRICT;
