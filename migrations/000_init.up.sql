CREATE TABLE IF NOT EXISTS zones (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	name TEXT NOT NULL UNIQUE
) STRICT;
CREATE INDEX zones_index ON zones(name);

CREATE TABLE IF NOT EXISTS records (
	id INTEGER PRIMARY KEY AUTOINCREMENT,
	zoneid INTEGER NOT NULL,
	subdomain TEXT NOT NULL,
	ttl INTEGER NOT NULL,
	class TEXT NOT NULL,
	type TEXT NOT NULL,
	data TEXT NOT NULL,
	FOREIGN KEY(zoneid) REFERENCES zones(id),
	UNIQUE (zoneid, subdomain, class, type),
	CHECK (ttl >= 0 AND ttl <= 4294967295)
) STRICT;
CREATE INDEX records_index ON records(zoneid, subdomain, class, type, ttl, data);
