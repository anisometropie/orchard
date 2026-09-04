use std::{collections::BTreeMap, env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=db/migrations");
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let migration_dir = manifest_dir.join("db/migrations");
    let down_migration_dir = migration_dir.join("down");
    let mut migrations = BTreeMap::new();
    let mut down_migrations = BTreeMap::new();

    for entry in fs::read_dir(&migration_dir).expect("the migration directory should be readable") {
        let entry = entry.expect("a migration directory entry should be readable");
        let file_name = entry
            .file_name()
            .into_string()
            .expect("migration filenames should be UTF-8");
        if !file_name.ends_with(".sql") {
            continue;
        }
        let (version, name) = migration_metadata(&file_name);
        assert_ne!(
            version, 9,
            "migration 009 is permanently retired because it contained hardcoded data"
        );
        assert!(
            migrations.insert(version, (name, file_name)).is_none(),
            "migration versions must be unique"
        );
    }
    assert!(!migrations.is_empty(), "at least one migration is required");

    if down_migration_dir.exists() {
        for entry in fs::read_dir(&down_migration_dir)
            .expect("the down migration directory should be readable")
        {
            let entry = entry.expect("a down migration directory entry should be readable");
            let file_name = entry
                .file_name()
                .into_string()
                .expect("down migration filenames should be UTF-8");
            if !file_name.ends_with(".sql") {
                continue;
            }
            let (version, name) = migration_metadata(&file_name);
            assert!(
                down_migrations.insert(version, (name, file_name)).is_none(),
                "down migration versions must be unique"
            );
        }
    }

    let mut generated = String::from("const MIGRATIONS: &[Migration] = &[\n");
    for (version, (name, file_name)) in migrations {
        let down_sql = match down_migrations.remove(&version) {
            Some((down_name, down_file_name)) => {
                assert_eq!(
                    down_name, name,
                    "up and down migration names must match for version {version}"
                );
                format!(
                    "Some(include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/db/migrations/down/{down_file_name}\")))"
                )
            }
            None => "None".into(),
        };
        generated.push_str(&format!(
            "    Migration {{ version: {version}, name: \"{name}\", sql: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/db/migrations/{file_name}\")), down_sql: {down_sql} }},\n"
        ));
    }
    assert!(
        down_migrations.is_empty(),
        "every down migration must have a matching up migration"
    );
    generated.push_str("];\n");
    let output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("embedded_migrations.rs");
    fs::write(output, generated).expect("embedded migration metadata should be generated");
}

fn migration_metadata(file_name: &str) -> (u32, String) {
    let stem = file_name
        .strip_suffix(".sql")
        .expect("only SQL migration files should be parsed");
    let (version, name) = stem
        .split_once('_')
        .expect("migration filenames must use NNN_name.sql");
    let version = version
        .parse::<u32>()
        .expect("migration versions must be positive integers");
    assert!(version > 0, "migration versions must be positive integers");
    assert!(
        !name.is_empty()
            && name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_'),
        "migration names must contain only ASCII letters, digits, and underscores"
    );
    (version, name.into())
}
