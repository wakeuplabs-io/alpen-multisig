ALTER TABLE proposals
  ADD COLUMN target_action_id TEXT REFERENCES proposals(action_id),
  ADD COLUMN activation_height BIGINT;

CREATE INDEX proposals_target_action_id_idx ON proposals(target_action_id);
