ALTER TABLE proposals ADD COLUMN broadcast_claimed_at TIMESTAMPTZ;

-- Rows already in flight keep a claim time, so the gate still treats them as taken.
-- `updated_at` is the only timestamp those rows have. A `commit_broadcasted` row
-- with no txids whose `updated_at` is already older than the reclaim window
-- (600s, see CLAIM_STALE_AFTER) becomes reclaimable on deploy.
UPDATE proposals
SET broadcast_claimed_at = updated_at
WHERE broadcast_status IN ('commit_broadcasted', 'commit_confirmed', 'reveal_broadcasted');
