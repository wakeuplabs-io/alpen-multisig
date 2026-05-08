ALTER TABLE proposals
    ADD COLUMN broadcast_status TEXT NOT NULL DEFAULT 'idle',
    ADD COLUMN commit_txid      TEXT,
    ADD COLUMN reveal_txid      TEXT,
    ADD COLUMN broadcast_error  TEXT;
