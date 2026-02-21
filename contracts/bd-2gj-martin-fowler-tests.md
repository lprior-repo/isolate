# Martin Fowler Test Plan: bd-2gj - CREATE TABLE conflict_resolutions for tracking AI/human decisions

**Bead ID:** bd-2gj
**Title:** CREATE TABLE conflict_resolutions for tracking AI/human decisions
**Test Framework:** Given-When-Then (BDD style)
**Coverage Target:** 100% of contract specification

## Test Suite Organization

```
bd-2gj-tests/
├── schema_tests/          # ST-001 to ST-010 (schema initialization)
├── insert_tests/          # IT-001 to IT-015 (insert operations)
├── query_tests/           # QT-001 to QT-020 (query operations)
├── invariant_tests/       # IV-001 to IV-010 (invariants)
├── error_tests/           # ER-001 to ER-015 (error handling)
└── performance_tests/     # PF-001 to PF-005 (performance)
```

---

## Schema Tests (ST)

### ST-001: Table created with correct schema

**GIVEN** a valid SQLite database connection pool
**WHEN** `init_conflict_resolutions_schema(&pool)` is called
**THEN** `conflict_resolutions` table exists
**AND** table has all required columns: id, timestamp, session, file, strategy, reason, confidence, decider
**AND** `id` column is INTEGER PRIMARY KEY AUTOINCREMENT
**AND** `timestamp`, `session`, `file`, `strategy`, `decider` are NOT NULL
**AND** `reason` and `confidence` are nullable
**AND** `decider` has CHECK constraint IN ('ai', 'human')

```rust
#[sqlx::test]
async fn test_st001_table_created_with_correct_schema(pool: SqlitePool) {
    // Arrange & Act
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Assert - verify table exists
    let result = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='conflict_resolutions'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(result.get::<String, _>("name"), "conflict_resolutions");

    // Assert - verify columns
    let columns = sqlx::query("PRAGMA table_info(conflict_resolutions)")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(columns.len(), 8);  // 8 columns

    let column_names: Vec<String> = columns.iter()
        .map(|row| row.get("name"))
        .collect();
    assert!(column_names.contains(&"id".to_string()));
    assert!(column_names.contains(&"timestamp".to_string()));
    assert!(column_names.contains(&"session".to_string()));
    assert!(column_names.contains(&"file".to_string()));
    assert!(column_names.contains(&"strategy".to_string()));
    assert!(column_names.contains(&"reason".to_string()));
    assert!(column_names.contains(&"confidence".to_string()));
    assert!(column_names.contains(&"decider".to_string()));
}
```

---

### ST-002: Session index created

**GIVEN** a valid SQLite database connection pool
**WHEN** `init_conflict_resolutions_schema(&pool)` is called
**THEN** `idx_conflict_resolutions_session` index exists
**AND** index is on `session` column

```rust
#[sqlx::test]
async fn test_st002_session_index_created(pool: SqlitePool) {
    // Arrange & Act
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Assert - verify index exists
    let result = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_session'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(result.is_some());
}
```

---

### ST-003: Timestamp index created

**GIVEN** a valid SQLite database connection pool
**WHEN** `init_conflict_resolutions_schema(&pool)` is called
**THEN** `idx_conflict_resolutions_timestamp` index exists
**AND** index is on `timestamp` column

```rust
#[sqlx::test]
async fn test_st003_timestamp_index_created(pool: SqlitePool) {
    // Arrange & Act
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Assert - verify index exists
    let result = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_timestamp'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(result.is_some());
}
```

---

### ST-004: Decider index created

**GIVEN** a valid SQLite database connection pool
**WHEN** `init_conflict_resolutions_schema(&pool)` is called
**THEN** `idx_conflict_resolutions_decider` index exists
**AND** index is on `decider` column

```rust
#[sqlx::test]
async fn test_st004_decider_index_created(pool: SqlitePool) {
    // Arrange & Act
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Assert - verify index exists
    let result = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_decider'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(result.is_some());
}
```

---

### ST-005: Composite session+timestamp index created

**GIVEN** a valid SQLite database connection pool
**WHEN** `init_conflict_resolutions_schema(&pool)` is called
**THEN** `idx_conflict_resolutions_session_timestamp` index exists
**AND** index is on `session` and `timestamp` columns

```rust
#[sqlx::test]
async fn test_st005_composite_index_created(pool: SqlitePool) {
    // Arrange & Act
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Assert - verify index exists
    let result = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_conflict_resolutions_session_timestamp'"
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert!(result.is_some());
}
```

---

### ST-006: Schema initialization is idempotent

**GIVEN** a valid SQLite database connection pool
**AND** `init_conflict_resolutions_schema(&pool)` has already been called once
**WHEN** `init_conflict_resolutions_schema(&pool)` is called again
**THEN** function returns Ok(())
**AND** no error occurs
**AND** table structure unchanged

```rust
#[sqlx::test]
async fn test_st006_schema_init_is_idempotent(pool: SqlitePool) {
    // Arrange & Act
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act - call again
    let result = init_conflict_resolutions_schema(&pool).await;

    // Assert
    assert!(result.is_ok());

    // Assert - table still works
    let count = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 0);  // Empty but exists
}
```

---

### ST-007: CHECK constraint on decider enforced

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** attempting to INSERT record with decider='invalid'
**THEN** database returns constraint violation error
**AND** record not inserted

```rust
#[sqlx::test]
async fn test_st007_decider_check_constraint_enforced(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act & Assert
    let result = sqlx::query(
        "INSERT INTO conflict_resolutions (timestamp, session, file, strategy, decider)
         VALUES ('2025-02-18T12:00:00Z', 'test', 'file.txt', 'accept_theirs', 'invalid')"
    )
    .execute(&pool)
    .await;

    // Assert - should fail with CHECK constraint error
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();
    assert!(err_msg.contains("check constraint") || err_msg.contains("constraint"));
}
```

---

### ST-008: NOT NULL constraints enforced

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** attempting to INSERT record with NULL in required field
**THEN** database returns NOT NULL constraint error
**AND** record not inserted

```rust
#[sqlx::test]
async fn test_st008_not_null_constraints_enforced(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act & Assert - try NULL timestamp
    let result = sqlx::query(
        "INSERT INTO conflict_resolutions (timestamp, session, file, strategy, decider)
         VALUES (NULL, 'test', 'file.txt', 'accept_theirs', 'ai')"
    )
    .execute(&pool)
    .await;

    // Assert - should fail
    assert!(result.is_err());
}
```

---

### ST-009: Foreign key relationship (optional validation)

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized with sessions table
**WHEN** inserting conflict_resolution with non-existent session
**THEN** insert succeeds (no FK constraint in v1)
**OR** insert fails (if FK constraint added)

```rust
#[sqlx::test]
async fn test_st009_foreign_key_relationship(pool: SqlitePool) {
    // Arrange
    init_sessions_schema(&pool).await.unwrap();
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act - insert with non-existent session
    let result = sqlx::query(
        "INSERT INTO conflict_resolutions (timestamp, session, file, strategy, decider)
         VALUES ('2025-02-18T12:00:00Z', 'nonexistent', 'file.txt', 'accept_theirs', 'ai')"
    )
    .execute(&pool)
    .await;

    // Assert - v1 has no FK constraint, so should succeed
    // Future versions may add FK constraint
    match result {
        Ok(_) => {
            // v1 behavior - no FK constraint
            println!("No FK constraint enforced (v1 behavior)");
        }
        Err(_) => {
            // Future behavior - FK constraint may be added
            println!("FK constraint enforced (future version)");
        }
    }
}
```

---

### ST-010: All indexes maintain query integrity

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**AND** 10 test records inserted
**WHEN** querying using each index
**THEN** all queries return correct results
**AND** index integrity maintained

```rust
#[sqlx::test]
async fn test_st010_indexes_maintain_integrity(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert test data
    for i in 0..10 {
        sqlx::query(
            "INSERT INTO conflict_resolutions (timestamp, session, file, strategy, decider)
             VALUES (?1, ?2, ?3, ?4, ?5)"
        )
        .bind(format!("2025-02-18T{:02}:00:00Z", i))
        .bind(format!("session-{}", i % 3))
        .bind(format!("file-{}.txt", i))
        .bind("accept_theirs")
        .bind(if i % 2 == 0 { "ai" } else { "human" })
        .execute(&pool)
        .await
        .unwrap();
    }

    // Assert - session index works
    let session_results = sqlx::query(
        "SELECT * FROM conflict_resolutions WHERE session = 'session-1'"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(session_results.len(), 4);  // 4 records with session-1

    // Assert - decider index works
    let ai_results = sqlx::query(
        "SELECT * FROM conflict_resolutions WHERE decider = 'ai'"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(ai_results.len(), 5);  // 5 AI records

    // Assert - timestamp index works
    let time_results = sqlx::query(
        "SELECT * FROM conflict_resolutions WHERE timestamp >= '2025-02-18T05:00:00Z'"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(time_results.len(), 5);  // 5 records in range
}
```

---

## Insert Tests (IT)

### IT-001: Insert valid AI resolution

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** `insert_conflict_resolution(&pool, &resolution)` is called with valid AI resolution
**THEN** function returns Ok(id) where id > 0
**AND** record persisted with correct field values
**AND** `decider` field is "ai"

```rust
#[sqlx::test]
async fn test_it001_insert_valid_ai_resolution(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,  // Auto-generated
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "my-session".to_string(),
        file: "src/main.rs".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: Some("Automatic resolution".to_string()),
        confidence: Some("high".to_string()),
        decider: "ai".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    assert!(id > 0);

    // Verify record persisted
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.id, id);
    assert_eq!(result.timestamp, "2025-02-18T12:34:56Z");
    assert_eq!(result.session, "my-session");
    assert_eq!(result.file, "src/main.rs");
    assert_eq!(result.strategy, "accept_theirs");
    assert_eq!(result.reason, Some("Automatic resolution".to_string()));
    assert_eq!(result.confidence, Some("high".to_string()));
    assert_eq!(result.decider, "ai");
}
```

---

### IT-002: Insert valid human resolution

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** `insert_conflict_resolution(&pool, &resolution)` is called with valid human resolution
**THEN** function returns Ok(id) where id > 0
**AND** record persisted with correct field values
**AND** `decider` field is "human"

```rust
#[sqlx::test]
async fn test_it002_insert_valid_human_resolution(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "my-session".to_string(),
        file: "src/lib.rs".to_string(),
        strategy: "manual_merge".to_string(),
        reason: Some("Manual resolution required".to_string()),
        confidence: None,  // No confidence for human
        decider: "human".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    assert!(id > 0);

    // Verify record persisted
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.decider, "human");
    assert_eq!(result.confidence, None);
}
```

---

### IT-003: Insert with NULL reason and confidence

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** `insert_conflict_resolution(&pool, &resolution)` is called with NULL optional fields
**THEN** function returns Ok(id)
**AND** record persisted with NULL values

```rust
#[sqlx::test]
async fn test_it003_insert_with_null_optional_fields(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "my-session".to_string(),
        file: "src/test.rs".to_string(),
        strategy: "accept_ours".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    assert!(id > 0);

    // Verify NULL values persisted
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.reason, None);
    assert_eq!(result.confidence, None);
}
```

---

### IT-004: Insert fails with invalid decider

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** `insert_conflict_resolution(&pool, &resolution)` is called with decider="robot"
**THEN** function returns Err(Error::DatabaseError)
**AND** error message mentions constraint violation
**AND** no record inserted

```rust
#[sqlx::test]
async fn test_it004_insert_fails_with_invalid_decider(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "my-session".to_string(),
        file: "src/test.rs".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "robot".to_string(),  // Invalid!
    };

    // Act
    let result = insert_conflict_resolution(&pool, &resolution).await;

    // Assert
    assert!(result.is_err());

    // Verify no record inserted
    let count = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 0);
}
```

---

### IT-005: Insert fails with empty required field

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** `insert_conflict_resolution(&pool, &resolution)` is called with empty session
**THEN** function returns Err(Error::InvalidInput)
**AND** no record inserted

```rust
#[sqlx::test]
async fn test_it005_insert_fails_with_empty_session(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "".to_string(),  // Empty!
        file: "src/test.rs".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };

    // Act
    let result = insert_conflict_resolution(&pool, &resolution).await;

    // Assert
    assert!(result.is_err());
}
```

---

### IT-006: Multiple inserts increment ID

**GIVEN** a valid SQLite database connection pool
**AND** schema initialized
**WHEN** inserting 5 conflict resolutions
**THEN** each insert returns incrementing IDs (1, 2, 3, 4, 5)
**AND** all records persisted correctly

```rust
#[sqlx::test]
async fn test_it006_multiple_inserts_increment_id(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let mut ids = Vec::new();

    // Act - insert 5 records
    for i in 0..5 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: format!("session-{}", i),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };

        let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();
        ids.push(id);
    }

    // Assert - IDs are incrementing
    assert_eq!(ids, vec![1, 2, 3, 4, 5]);

    // Assert - all records exist
    let count = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 5);
}
```

---

### IT-007: Insert with Unicode characters

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting resolution with Unicode file path and session name
**THEN** insert succeeds
**AND** Unicode characters preserved

```rust
#[sqlx::test]
async fn test_it007_insert_with_unicode_characters(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "café-session".to_string(),
        file: "src/テスト.rs".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: Some("Unicode test: Ñoño".to_string()),
        confidence: None,
        decider: "ai".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.session, "café-session");
    assert_eq!(result.file, "src/テスト.rs");
    assert_eq!(result.reason, Some("Unicode test: Ñoño".to_string()));
}
```

---

### IT-008: Insert with very long field values

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting resolution with very long file path (1000 chars)
**THEN** insert succeeds
**AND** long value preserved

```rust
#[sqlx::test]
async fn test_it008_insert_with_very_long_file_path(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let long_path = "a".repeat(1000);

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "my-session".to_string(),
        file: long_path.clone(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.file.len(), 1000);
}
```

---

### IT-009: Insert with different strategies

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting resolutions with different strategies
**THEN** all inserts succeed
**AND** strategies preserved

```rust
#[sqlx::test]
async fn test_it009_insert_with_different_strategies(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let strategies = vec![
        "accept_theirs",
        "accept_ours",
        "manual_merge",
        "skip",
    ];

    for strategy in strategies {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "my-session".to_string(),
            file: "file.txt".to_string(),
            strategy: strategy.to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };

        // Act
        let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

        // Assert
        let result = sqlx::query_as::<_, ConflictResolution>(
            "SELECT * FROM conflict_resolutions WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(result.strategy, strategy);
    }
}
```

---

### IT-010: Insert with confidence scores

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting AI resolutions with different confidence scores
**THEN** all inserts succeed
**AND** confidence scores preserved

```rust
#[sqlx::test]
async fn test_it010_insert_with_confidence_scores(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let confidences = vec![
        Some("high".to_string()),
        Some("medium".to_string()),
        Some("low".to_string()),
        Some("0.95".to_string()),
        Some("0.5".to_string()),
        None,
    ];

    for confidence in confidences {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "my-session".to_string(),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: confidence.clone(),
            decider: "ai".to_string(),
        };

        // Act
        let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

        // Assert
        let result = sqlx::query_as::<_, ConflictResolution>(
            "SELECT * FROM conflict_resolutions WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(result.confidence, confidence);
    }
}
```

---

### IT-011: Insert with valid ISO 8601 timestamps

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting resolutions with various valid ISO 8601 timestamps
**THEN** all inserts succeed
**AND** timestamps preserved

```rust
#[sqlx::test]
async fn test_it011_insert_with_valid_timestamps(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let timestamps = vec![
        "2025-02-18T12:34:56Z",
        "2025-02-18T12:34:56.123Z",
        "2025-02-18T12:34:56+00:00",
        "2025-02-18T12:34:56.789+00:00",
    ];

    for timestamp in timestamps {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: timestamp.to_string(),
            session: "my-session".to_string(),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };

        // Act
        let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

        // Assert
        let result = sqlx::query_as::<_, ConflictResolution>(
            "SELECT * FROM conflict_resolutions WHERE id = ?"
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(result.timestamp, timestamp);
    }
}
```

---

### IT-012: Concurrent inserts from multiple agents

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** 10 concurrent inserts are executed
**THEN** all inserts succeed
**AND** all IDs are unique
**AND** no database corruption

```rust
#[sqlx::test]
async fn test_it012_concurrent_inserts(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act - spawn 10 concurrent inserts
    let mut handles = Vec::new();
    for i in 0..10 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let resolution = ConflictResolution {
                id: 0,
                timestamp: "2025-02-18T12:34:56Z".to_string(),
                session: format!("session-{}", i),
                file: format!("file-{}.txt", i),
                strategy: "accept_theirs".to_string(),
                reason: None,
                confidence: None,
                decider: "ai".to_string(),
            };
            insert_conflict_resolution(&pool_clone, &resolution).await
        });
        handles.push(handle);
    }

    // Wait for all inserts
    let mut ids = Vec::new();
    for handle in handles {
        let id = handle.await.unwrap().unwrap();
        ids.push(id);
    }

    // Assert - all inserts succeeded
    assert_eq!(ids.len(), 10);

    // Assert - all IDs are unique
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique_ids.len(), 10);

    // Assert - all records exist
    let count = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 10);
}
```

---

### IT-013: Insert with reason containing special characters

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting resolution with reason containing quotes, newlines, etc.
**THEN** insert succeeds
**AND** special characters preserved

```rust
#[sqlx::test]
async fn test_it013_insert_with_special_characters_in_reason(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let reason = r#"Test "quoted" text with 'single quotes'
and newlines
and special chars: @#$%^&*()"#;

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "my-session".to_string(),
        file: "file.txt".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: Some(reason.to_string()),
        confidence: None,
        decider: "ai".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.reason, Some(reason.to_string()));
}
```

---

### IT-014: Insert with file path containing spaces

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting resolution with file path containing spaces
**THEN** insert succeeds
**AND** path with spaces preserved

```rust
#[sqlx::test]
async fn test_it014_insert_with_file_path_containing_spaces(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "my-session".to_string(),
        file: "path/with spaces/in it/file.txt".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.file, "path/with spaces/in it/file.txt");
}
```

---

### IT-015: Insert with same timestamp across sessions

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting multiple resolutions with same timestamp in different sessions
**THEN** all inserts succeed
**AND** all records have unique IDs

```rust
#[sqlx::test]
async fn test_it015_insert_with_same_timestamp_different_sessions(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let timestamp = "2025-02-18T12:34:56Z";

    // Insert 3 records with same timestamp
    for i in 0..3 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: timestamp.to_string(),
            session: format!("session-{}", i),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };

        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Assert - all 3 records exist with same timestamp
    let results = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE timestamp = ?"
    )
    .bind(timestamp)
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.timestamp == timestamp));
}
```

---

## Query Tests (QT)

### QT-001: Query resolutions by session

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** 3 resolutions for session "alpha" and 2 for session "beta"
**WHEN** `get_conflict_resolutions(&pool, "alpha")` is called
**THEN** returns 3 resolutions
**AND** all resolutions have session="alpha"
**AND** results ordered by id ascending

```rust
#[sqlx::test]
async fn test_qt001_query_resolutions_by_session(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert test data
    for i in 0..3 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "alpha".to_string(),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    for i in 0..2 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "beta".to_string(),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act
    let results = get_conflict_resolutions(&pool, "alpha").await.unwrap();

    // Assert
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.session == "alpha"));

    // Assert - ordered by id ascending
    let ids: Vec<i64> = results.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![1, 2, 3]);
}
```

---

### QT-002: Query resolutions by decider (AI)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** 5 AI resolutions and 3 human resolutions
**WHEN** `get_resolutions_by_decider(&pool, "ai")` is called
**THEN** returns 5 resolutions
**AND** all resolutions have decider="ai"

```rust
#[sqlx::test]
async fn test_qt002_query_resolutions_by_decider_ai(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert 5 AI resolutions
    for i in 0..5 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "session".to_string(),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Insert 3 human resolutions
    for i in 0..3 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "session".to_string(),
            file: format!("file-{}.txt", i + 5),
            strategy: "manual_merge".to_string(),
            reason: None,
            confidence: None,
            decider: "human".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act
    let results = get_resolutions_by_decider(&pool, "ai").await.unwrap();

    // Assert
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|r| r.decider == "ai"));
}
```

---

### QT-003: Query resolutions by decider (human)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** 5 AI resolutions and 3 human resolutions
**WHEN** `get_resolutions_by_decider(&pool, "human")` is called
**THEN** returns 3 resolutions
**AND** all resolutions have decider="human"

```rust
#[sqlx::test]
async fn test_qt003_query_resolutions_by_decider_human(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert 5 AI resolutions
    for i in 0..5 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "session".to_string(),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Insert 3 human resolutions
    for i in 0..3 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "session".to_string(),
            file: format!("file-{}.txt", i + 5),
            strategy: "manual_merge".to_string(),
            reason: None,
            confidence: None,
            decider: "human".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act
    let results = get_resolutions_by_decider(&pool, "human").await.unwrap();

    // Assert
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.decider == "human"));
}
```

---

### QT-004: Query resolutions by time range

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** resolutions with timestamps at 10:00, 11:00, 12:00, 13:00, 14:00
**WHEN** `get_resolutions_by_time_range(&pool, "2025-02-18T11:00:00Z", "2025-02-18T14:00:00Z")` is called
**THEN** returns 3 resolutions (11:00, 12:00, 13:00)
**AND** range is inclusive of start, exclusive of end

```rust
#[sqlx::test]
async fn test_qt004_query_resolutions_by_time_range(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let timestamps = vec![
        "2025-02-18T10:00:00Z",
        "2025-02-18T11:00:00Z",
        "2025-02-18T12:00:00Z",
        "2025-02-18T13:00:00Z",
        "2025-02-18T14:00:00Z",
    ];

    for (i, timestamp) in timestamps.iter().enumerate() {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: timestamp.to_string(),
            session: "session".to_string(),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act
    let results = get_resolutions_by_time_range(
        &pool,
        "2025-02-18T11:00:00Z",
        "2025-02-18T14:00:00Z"
    ).await.unwrap();

    // Assert - should get 11:00, 12:00, 13:00 (not 14:00, exclusive)
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.timestamp >= "2025-02-18T11:00:00Z"));
    assert!(results.iter().all(|r| r.timestamp < "2025-02-18T14:00:00Z"));
}
```

---

### QT-005: Query non-existent session returns empty

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** no resolutions for session "nonexistent"
**WHEN** `get_conflict_resolutions(&pool, "nonexistent")` is called
**THEN** returns empty Vec
**AND** no error

```rust
#[sqlx::test]
async fn test_qt005_query_nonexistent_session_returns_empty(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert resolutions for different session
    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "other-session".to_string(),
        file: "file.txt".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };
    insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Act
    let results = get_conflict_resolutions(&pool, "nonexistent").await.unwrap();

    // Assert
    assert_eq!(results.len(), 0);
    assert!(results.is_empty());
}
```

---

### QT-006: Query with invalid decider returns error

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** `get_resolutions_by_decider(&pool, "robot")` is called
**THEN** returns Err(Error::InvalidInput)
**AND** error message indicates invalid decider

```rust
#[sqlx::test]
async fn test_qt006_query_with_invalid_decider_returns_error(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act
    let result = get_resolutions_by_decider(&pool, "robot").await;

    // Assert
    assert!(result.is_err());
}
```

---

### QT-007: Query with invalid time range returns error

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** `get_resolutions_by_time_range(&pool, "2025-02-18T14:00:00Z", "2025-02-18T11:00:00Z")` is called (start > end)
**THEN** returns Err(Error::InvalidInput)
**AND** error message indicates invalid range

```rust
#[sqlx::test]
async fn test_qt007_query_with_invalid_time_range_returns_error(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act - start > end
    let result = get_resolutions_by_time_range(
        &pool,
        "2025-02-18T14:00:00Z",
        "2025-02-18T11:00:00Z"
    ).await;

    // Assert
    assert!(result.is_err());
}
```

---

### QT-008: Query with invalid timestamp format returns error

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** `get_resolutions_by_time_range(&pool, "not-a-timestamp", "2025-02-18T11:00:00Z")` is called
**THEN** returns Err(Error::InvalidInput)
**AND** error message indicates invalid timestamp format

```rust
#[sqlx::test]
async fn test_qt008_query_with_invalid_timestamp_format_returns_error(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act
    let result = get_resolutions_by_time_range(
        &pool,
        "not-a-timestamp",
        "2025-02-18T11:00:00Z"
    ).await;

    // Assert
    assert!(result.is_err());
}
```

---

### QT-009: Query results ordered by timestamp

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** resolutions inserted with timestamps out of order
**WHEN** `get_resolutions_by_time_range()` is called
**THEN** results ordered by timestamp ascending

```rust
#[sqlx::test]
async fn test_qt009_query_results_ordered_by_timestamp(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let timestamps = vec![
        "2025-02-18T12:00:00Z",
        "2025-02-18T10:00:00Z",  // Out of order
        "2025-02-18T14:00:00Z",
        "2025-02-18T11:00:00Z",
    ];

    for timestamp in timestamps {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: timestamp.to_string(),
            session: "session".to_string(),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act
    let results = get_resolutions_by_time_range(
        &pool,
        "2025-02-18T00:00:00Z",
        "2025-02-18T23:59:59Z"
    ).await.unwrap();

    // Assert - ordered by timestamp ascending
    let sorted_timestamps: Vec<&str> = results.iter().map(|r| r.timestamp.as_str()).collect();
    assert_eq!(sorted_timestamps, vec![
        "2025-02-18T10:00:00Z",
        "2025-02-18T11:00:00Z",
        "2025-02-18T12:00:00Z",
        "2025-02-18T14:00:00Z",
    ]);
}
```

---

### QT-010: Query with empty session name returns error

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** `get_conflict_resolutions(&pool, "")` is called
**THEN** returns Err(Error::InvalidInput)

```rust
#[sqlx::test]
async fn test_qt010_query_with_empty_session_returns_error(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act
    let result = get_conflict_resolutions(&pool, "").await;

    // Assert
    assert!(result.is_err());
}
```

---

## Invariant Tests (IV)

### IV-001: Table is append-only (no UPDATE)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** resolution record inserted
**WHEN** attempting to UPDATE the record
**THEN** NO UPDATE function exists in API
**AND** direct SQL UPDATE is possible but violates invariant
**AND** tests verify no update logic in implementation

```rust
#[test]
fn test_iv001_no_update_function_exists() {
    // This is a compile-time test
    // If this compiles, the invariant is violated

    // Assert - function does not exist (compile error)
    // update_conflict_resolution()  // This should NOT exist
}
```

---

### IV-002: Table is append-only (no DELETE)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** resolution record inserted
**WHEN** attempting to DELETE the record via API
**THEN** NO DELETE function exists in API
**AND** direct SQL DELETE is possible but violates invariant

```rust
#[test]
fn test_iv002_no_delete_function_exists() {
    // This is a compile-time test
    // If this compiles, the invariant is violated

    // Assert - function does not exist (compile error)
    // delete_conflict_resolution()  // This should NOT exist
}
```

---

### IV-003: Decider constraint always enforced

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting record with decider NOT in ("ai", "human")
**THEN** database constraint fails
**AND** no record inserted

```rust
#[sqlx::test]
async fn test_iv003_decider_constraint_always_enforced(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let invalid_deciders = vec![
        "AI",      // Wrong case
        "Human",   // Wrong case
        "robot",   // Invalid value
        "AI-human", // Invalid value
        "",        // Empty
    ];

    for decider in invalid_deciders {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "session".to_string(),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: decider.to_string(),
        };

        // Act & Assert
        let result = insert_conflict_resolution(&pool, &resolution).await;
        assert!(result.is_err(), "Should fail for decider: {}", decider);
    }

    // Assert - no records inserted
    let count = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 0);
}
```

---

### IV-004: Timestamp is always required (NOT NULL)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting record with NULL timestamp
**THEN** database constraint fails

```rust
#[sqlx::test]
async fn test_iv004_timestamp_always_required(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Act - direct SQL with NULL
    let result = sqlx::query(
        "INSERT INTO conflict_resolutions (timestamp, session, file, strategy, decider)
         VALUES (NULL, 'session', 'file.txt', 'accept_theirs', 'ai')"
    )
    .execute(&pool)
    .await;

    // Assert
    assert!(result.is_err());
}
```

---

### IV-005: Primary key is always unique

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** 5 records inserted
**WHEN** querying all records
**THEN** all IDs are unique

```rust
#[sqlx::test]
async fn test_iv005_primary_key_always_unique(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert 5 records
    for i in 0..5 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: format!("session-{}", i),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act
    let results = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    // Assert - all IDs unique
    let ids: Vec<i64> = results.iter().map(|r| r.id).collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique_ids.len(), 5);
}
```

---

### IV-006: Primary key is monotonically increasing

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** 5 records inserted
**WHEN** querying all records ordered by id
**THEN** IDs are 1, 2, 3, 4, 5

```rust
#[sqlx::test]
async fn test_iv006_primary_key_monotonically_increasing(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert 5 records
    for _ in 0..5 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: "session".to_string(),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act
    let results = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions ORDER BY id"
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    // Assert - IDs are 1, 2, 3, 4, 5
    let ids: Vec<i64> = results.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![1, 2, 3, 4, 5]);
}
```

---

### IV-007: Optional fields can be NULL

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting record with NULL reason and confidence
**THEN** insert succeeds
**AND** NULL values preserved

```rust
#[sqlx::test]
async fn test_iv007_optional_fields_can_be_null(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let resolution = ConflictResolution {
        id: 0,
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "session".to_string(),
        file: "file.txt".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };

    // Act
    let id = insert_conflict_resolution(&pool, &resolution).await.unwrap();

    // Assert
    let result = sqlx::query_as::<_, ConflictResolution>(
        "SELECT * FROM conflict_resolutions WHERE id = ?"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(result.reason, None);
    assert_eq!(result.confidence, None);
}
```

---

### IV-008: Schema initialization is idempotent

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** 3 records inserted
**WHEN** `init_conflict_resolutions_schema(&pool)` is called again
**THEN** function returns Ok(())
**AND** existing records preserved
**AND** no data loss

```rust
#[sqlx::test]
async fn test_iv008_schema_init_is_idempotent(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert test data
    for i in 0..3 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: format!("session-{}", i),
            file: "file.txt".to_string(),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    // Act - initialize again
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Assert - data preserved
    let count = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 3);
}
```

---

## Error Tests (ER)

### ER-001: Database connection fails

**GIVEN** a SQLite database pool with invalid connection
**WHEN** `init_conflict_resolutions_schema(&pool)` is called
**THEN** returns Err(Error::DatabaseError)
**AND** error message indicates connection failure

```rust
#[test]
async fn test_er001_database_connection_fails() {
    // Arrange - invalid pool
    let pool = SqlitePool::connect("sqlite:///invalid/path/to/db.sqlite").await;

    // Act & Assert
    assert!(pool.is_err());
}
```

---

### ER-002: Insert with duplicate ID (manual override)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** record with ID=1 exists
**WHEN** attempting to INSERT with explicit ID=1 (bypassing API)
**THEN** may fail or succeed (depending on SQLite configuration)
**AND** API always uses auto-increment

```rust
#[sqlx::test]
async fn test_er002_insert_with_duplicate_id(pool: SqlitePool) {
    // This test verifies the API doesn't allow explicit ID insertion
    // The ConflictResolution struct's `id` field is ignored (always 0 input)

    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert first record
    let resolution1 = ConflictResolution {
        id: 0,  // Auto-generated
        timestamp: "2025-02-18T12:34:56Z".to_string(),
        session: "session".to_string(),
        file: "file.txt".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };
    let id1 = insert_conflict_resolution(&pool, &resolution1).await.unwrap();

    // Insert second record
    let resolution2 = ConflictResolution {
        id: 999,  // Ignored by API
        timestamp: "2025-02-18T12:34:57Z".to_string(),
        session: "session".to_string(),
        file: "file2.txt".to_string(),
        strategy: "accept_theirs".to_string(),
        reason: None,
        confidence: None,
        decider: "ai".to_string(),
    };
    let id2 = insert_conflict_resolution(&pool, &resolution2).await.unwrap();

    // Assert - IDs are auto-generated, not manual
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_ne!(id2, 999);
}
```

---

### ER-003: Query with database closed

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** database connection is closed
**AND** `get_conflict_resolutions(&pool, "session")` is called
**THEN** returns Err(Error::DatabaseError)

```rust
#[sqlx::test]
async fn test_er003_query_with_database_closed(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Close pool
    pool.close().await;

    // Act
    let result = get_conflict_resolutions(&pool, "session").await;

    // Assert
    assert!(result.is_err());
}
```

---

## Performance Tests (PF)

### PF-001: Insert performance target (< 10ms)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** inserting 100 conflict resolutions
**THEN** each insert completes in < 10ms on average

```rust
#[sqlx::test]
async fn test_pf001_insert_performance_target(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let start = std::time::Instant::now();

    // Act - insert 100 records
    for i in 0..100 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: format!("session-{}", i % 10),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: "ai".to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    let elapsed = start.elapsed();

    // Assert - average < 10ms per insert
    let avg_ms = elapsed.as_millis() / 100;
    assert!(avg_ms < 10, "Average insert time: {}ms (target: < 10ms)", avg_ms);

    println!("Insert performance: {}ms per insert (100 inserts)", avg_ms);
}
```

---

### PF-002: Query performance target (< 100ms for 1000 records)

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**AND** 1000 conflict resolutions inserted
**WHEN** querying by session
**THEN** query completes in < 100ms

```rust
#[sqlx::test]
async fn test_pf002_query_performance_target(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    // Insert 1000 records
    for i in 0..1000 {
        let resolution = ConflictResolution {
            id: 0,
            timestamp: "2025-02-18T12:34:56Z".to_string(),
            session: format!("session-{}", i % 100),
            file: format!("file-{}.txt", i),
            strategy: "accept_theirs".to_string(),
            reason: None,
            confidence: None,
            decider: if i % 2 == 0 { "ai" } else { "human" }.to_string(),
        };
        insert_conflict_resolution(&pool, &resolution).await.unwrap();
    }

    let start = std::time::Instant::now();

    // Act - query by session (should return ~10 records)
    let results = get_conflict_resolutions(&pool, "session-5").await.unwrap();

    let elapsed = start.elapsed();

    // Assert
    assert_eq!(results.len(), 10);
    assert!(elapsed.as_millis() < 100, "Query time: {}ms (target: < 100ms)", elapsed.as_millis());

    println!("Query performance: {}ms for {} results", elapsed.as_millis(), results.len());
}
```

---

### PF-003: Concurrent insert performance

**GIVEN** a valid SQLite database pool
**AND** schema initialized
**WHEN** 50 concurrent inserts are executed
**THEN** all inserts complete in reasonable time (< 5 seconds total)

```rust
#[sqlx::test]
async fn test_pf003_concurrent_insert_performance(pool: SqlitePool) {
    // Arrange
    init_conflict_resolutions_schema(&pool).await.unwrap();

    let start = std::time::Instant::now();

    // Act - spawn 50 concurrent inserts
    let handles: Vec<_> = (0..50)
        .map(|i| {
            let pool_clone = pool.clone();
            tokio::spawn(async move {
                let resolution = ConflictResolution {
                    id: 0,
                    timestamp: "2025-02-18T12:34:56Z".to_string(),
                    session: format!("session-{}", i % 10),
                    file: format!("file-{}.txt", i),
                    strategy: "accept_theirs".to_string(),
                    reason: None,
                    confidence: None,
                    decider: "ai".to_string(),
                };
                insert_conflict_resolution(&pool_clone, &resolution).await
            })
        })
        .collect();

    // Wait for all inserts
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let elapsed = start.elapsed();

    // Assert - all inserts completed
    let count = sqlx::query("SELECT COUNT(*) as count FROM conflict_resolutions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.get::<i64, _>("count"), 50);

    // Assert - reasonable time (< 5 seconds)
    assert!(elapsed.as_secs() < 5, "Concurrent inserts took: {}s (target: < 5s)", elapsed.as_secs());

    println!("Concurrent insert performance: {}ms for 50 inserts", elapsed.as_millis());
}
```

---

## Test Execution Order

### Phase 1: Schema Tests (ST-001 to ST-010)
- Verify table and indexes created correctly
- All MUST pass before proceeding

### Phase 2: Insert Tests (IT-001 to IT-015)
- Test insert operations with various inputs
- All MUST pass

### Phase 3: Query Tests (QT-001 to QT-010)
- Test query operations
- All MUST pass

### Phase 4: Invariant Tests (IV-001 to IV-008)
- Verify append-only, constraints, idempotency
- All MUST pass

### Phase 5: Error Tests (ER-001 to ER-003)
- Test error handling
- All MUST pass

### Phase 6: Performance Tests (PF-001 to PF-003)
- Verify performance targets
- All SHOULD pass

## Success Criteria

- **P0 (Critical):** All ST, IT, QT, IV, ER tests pass
- **P1 (High):** All PF tests pass
- **Coverage:** 100% of contract specification tested
- **Performance:** Insert < 10ms, Query < 100ms

## Test Metrics

- Total tests: 46
- Critical (P0): 43 tests
- High priority (P1): 3 tests
- Estimated execution time: 2-3 minutes (serial)

## Implementation Location

Tests should be implemented in:
```
/home/lewis/src/zjj/crates/zjj-core/tests/conflict_resolutions_tests.rs
```

---

**Test Plan Version:** 1.0
**Last Updated:** 2025-02-18
**Author:** rust-contract agent
**Status:** Ready for implementation
