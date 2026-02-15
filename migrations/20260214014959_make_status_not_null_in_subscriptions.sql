BEGIN;
    -- Include a default value in previous records
    UPDATE subscriptions
        SET status = 'confirmed'
    WHERE status IS NULL;

    -- Status column as required
    ALTER TABLE subscriptions ALTER COLUMN status SET NOT NULL;
COMMIT;
