CREATE TABLE solo_recovery_receipts (
 session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
 causation_id TEXT NOT NULL,
 request_json TEXT NOT NULL CHECK(json_valid(request_json)),
 result_json TEXT NOT NULL CHECK(json_valid(result_json)),
 PRIMARY KEY(session_id,causation_id)
);
