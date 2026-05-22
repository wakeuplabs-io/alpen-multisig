ALTER TABLE proposals
ADD COLUMN required_signatures SMALLINT NOT NULL DEFAULT 1;

ALTER TABLE proposals
ALTER COLUMN required_signatures DROP DEFAULT;
