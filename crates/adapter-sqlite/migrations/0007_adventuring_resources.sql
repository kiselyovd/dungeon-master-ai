CREATE TABLE adventuring_states (
 session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,
 elapsed_seconds INTEGER NOT NULL CHECK(elapsed_seconds >= 0)
);
CREATE TABLE rest_operations (
 session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
 causation_id TEXT NOT NULL,
 request_json TEXT NOT NULL,
 result_json TEXT NOT NULL,
 PRIMARY KEY(session_id,causation_id)
);
