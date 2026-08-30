BEGIN;

WITH raspberry_windows(cultivar, start_month, start_day, end_month, end_day) AS (
    VALUES
        ('Autumn Bliss',       8,  1, 10, 20),
        ('Autumn First',       7, 20,  9, 30),
        ('Bohème',             6, 10,  6, 20),
        ('Bohème',             8,  1,  9, 20),
        ('EMR 201201',         8,  1, 10, 15),
        ('Fall Gold',          6, 15,  7, 10),
        ('Fall Gold',          8,  5, 10, 20),
        ('Glen Ample',         6, 15,  7, 25),
        ('Heritage',           6, 15,  7,  5),
        ('Heritage',           8, 10, 10, 20),
        ('Jdeboer005',         8,  1, 10, 15),
        ('MA 2920',            8,  1, 10, 15),
        ('Malling Happy',      8,  1, 10, 20),
        ('Malling Promise',    6, 10,  7, 15),
        ('Paris',              6, 15,  7, 20),
        ('Paris',              9,  5, 10, 25),
        ('Sucrée de Metz',     6, 15,  7, 25),
        ('Surprise d’Automne', 8,  5, 10, 15),
        ('Zeva',               6, 15,  7, 15),
        ('Zeva',               8, 10, 10, 15)
)
INSERT INTO plant_harvest_windows (
    plant_identity_id, cultivar_id,
    start_month, start_day, end_month, end_day
)
SELECT
    cultivar.plant_identity_id,
    cultivar.id,
    raspberry.start_month,
    raspberry.start_day,
    raspberry.end_month,
    raspberry.end_day
FROM raspberry_windows raspberry
JOIN plant_cultivars cultivar ON cultivar.cultivar = raspberry.cultivar
JOIN plant_identities identity ON identity.id = cultivar.plant_identity_id
WHERE identity.botanical_taxon->'Named'->>'genus' = 'Rubus'
  AND identity.botanical_taxon->'Named'->>'species' = 'idaeus'
ON CONFLICT DO NOTHING;

COMMIT;
