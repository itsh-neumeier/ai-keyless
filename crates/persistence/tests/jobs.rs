//! Queue-Tests gegen eine echte PostgreSQL-Instanz.

use akl_core::job::JobState;
use akl_persistence::{
    Database,
    jobs::{ClaimOutcome, EnqueueOutcome, JobRecord},
    testing::{fresh_schema, test_database_url},
};
use serde_json::json;

async fn enqueue(db: &Database, key: &str, priority: i16, max_attempts: i32) -> JobRecord {
    match db
        .enqueue_job(key, "test", &json!({"k": key}), priority, max_attempts)
        .await
        .unwrap()
    {
        EnqueueOutcome::Created(job) => job,
        EnqueueOutcome::Duplicate(_) => panic!("unerwartetes Duplikat"),
    }
}

async fn claim(db: &Database, worker: &str) -> JobRecord {
    match db.claim_job(worker, 60, 1000).await.unwrap() {
        ClaimOutcome::Claimed(job) => *job,
        other => panic!("Job erwartet, erhalten: {other:?}"),
    }
}

/// Rückt einen Job zeitlich vor (statt im Test zu warten).
async fn make_due(db: &Database, id: uuid::Uuid) {
    sqlx::query("UPDATE jobs SET next_attempt_at = now() - interval '1 second' WHERE id = $1")
        .bind(id)
        .execute(db.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn idempotency_priority_and_completion() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_jobs_basic").await;

    let low = enqueue(&db, "a", 0, 3).await;
    let high = enqueue(&db, "b", 5, 3).await;
    assert!(matches!(
        db.enqueue_job("a", "test", &json!({}), 0, 3).await.unwrap(),
        EnqueueOutcome::Duplicate(j) if j.id == low.id
    ));

    let first = claim(&db, "w1").await;
    assert_eq!(first.id, high.id, "höhere Priorität zuerst");
    assert_eq!((first.state, first.attempts), (JobState::Running, 1));

    // Ein fremder Worker darf den Job nicht abschließen.
    assert!(!db.complete_job(first.id, "w2", &json!({})).await.unwrap());
    assert!(
        db.complete_job(first.id, "w1", &json!({"ok": true}))
            .await
            .unwrap()
    );
    assert!(
        !db.complete_job(first.id, "w1", &json!({})).await.unwrap(),
        "kein doppelter Abschluss"
    );
    let done = db.get_job(first.id).await.unwrap().unwrap();
    assert_eq!(done.state, JobState::Completed);
    assert!(done.finished_at.is_some());

    assert_eq!(claim(&db, "w1").await.id, low.id);
    assert!(matches!(
        db.claim_job("w1", 60, 1000).await.unwrap(),
        ClaimOutcome::Empty
    ));
}

#[tokio::test]
async fn retries_with_backoff_then_dead_letter() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_jobs_retry").await;
    let job = enqueue(&db, "r", 0, 2).await;

    claim(&db, "w").await;
    assert_eq!(
        db.fail_job(job.id, "w", "timeout", true).await.unwrap(),
        Some(JobState::Retryable)
    );
    // Backoff: noch nicht fällig.
    assert!(matches!(
        db.claim_job("w", 60, 1000).await.unwrap(),
        ClaimOutcome::Empty
    ));
    let waiting = db.get_job(job.id).await.unwrap().unwrap();
    assert!(waiting.next_attempt_at > chrono::Utc::now() + chrono::Duration::seconds(20));

    make_due(&db, job.id).await;
    let again = claim(&db, "w").await;
    assert_eq!(again.attempts, 2);
    // Letzter Versuch scheitert → endgültig failed (Dead Letter).
    assert_eq!(
        db.fail_job(job.id, "w", "timeout", true).await.unwrap(),
        Some(JobState::Failed)
    );

    // Nicht wiederholbare Fehler gehen sofort auf failed.
    let other = enqueue(&db, "nr", 0, 5).await;
    claim(&db, "w").await;
    assert_eq!(
        db.fail_job(other.id, "w", "invalid_input", false)
            .await
            .unwrap(),
        Some(JobState::Failed)
    );

    // Operator: failed → queued gibt genau einen weiteren Versuch frei.
    assert!(db.requeue_job(job.id).await.unwrap());
    let requeued = db.get_job(job.id).await.unwrap().unwrap();
    assert_eq!(
        (requeued.state, requeued.max_attempts),
        (JobState::Queued, 3)
    );
    assert!(
        !db.requeue_job(job.id).await.unwrap(),
        "queued kann nicht erneut eingereiht werden"
    );
}

#[tokio::test]
async fn hourly_limit_is_durable_and_rolling() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_jobs_limit").await;
    for i in 0..3 {
        enqueue(&db, &format!("l{i}"), 0, 3).await;
    }
    for _ in 0..2 {
        assert!(matches!(
            db.claim_job("w", 60, 2).await.unwrap(),
            ClaimOutcome::Claimed(_)
        ));
    }
    let ClaimOutcome::RateLimited { retry_at } = db.claim_job("w", 60, 2).await.unwrap() else {
        panic!("Limit erwartet");
    };
    assert!(retry_at > chrono::Utc::now() + chrono::Duration::minutes(59));

    // Limit ist in der DB gespeichert: ein "neuer Prozess" (neuer Pool) sieht es auch.
    let fresh = fresh_db_same_schema(&url, "akl_jobs_limit").await;
    assert!(matches!(
        fresh.claim_job("w2", 60, 2).await.unwrap(),
        ClaimOutcome::RateLimited { .. }
    ));
    assert_eq!(fresh.queue_stats().await.unwrap().starts_last_hour, 2);

    // Rollierend: Starts älter als 1 h zählen nicht mehr.
    sqlx::query("UPDATE job_starts SET started_at = now() - interval '61 minutes'")
        .execute(db.pool())
        .await
        .unwrap();
    assert!(matches!(
        db.claim_job("w", 60, 2).await.unwrap(),
        ClaimOutcome::Claimed(_)
    ));
}

async fn fresh_db_same_schema(url: &str, schema: &str) -> Database {
    let schema = schema.to_owned();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |conn, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query(sqlx::AssertSqlSafe(format!("SET search_path TO {schema}")))
                    .execute(conn)
                    .await?;
                Ok(())
            })
        })
        .connect(url)
        .await
        .unwrap();
    Database::from_pool(pool)
}

#[tokio::test]
async fn expired_leases_recover_but_callbacks_become_uncertain() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_jobs_lease").await;
    let before_callback = enqueue(&db, "p", 0, 3).await;
    let in_callback = enqueue(&db, "c", 0, 3).await;
    let exhausted = enqueue(&db, "x", 0, 1).await;

    for _ in 0..3 {
        assert!(matches!(
            db.claim_job("dead-worker", 0, 1000).await.unwrap(),
            ClaimOutcome::Claimed(_)
        ));
    }
    assert!(
        db.mark_job_callback_phase(in_callback.id, "dead-worker")
            .await
            .unwrap()
    );
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let report = db.recover_expired_jobs().await.unwrap();
    assert_eq!((report.retried, report.uncertain), (2, 1));
    let state = |id| {
        let db = db.clone();
        async move { db.get_job(id).await.unwrap().unwrap().state }
    };
    assert_eq!(state(before_callback.id).await, JobState::Retryable);
    assert_eq!(state(in_callback.id).await, JobState::Uncertain);
    assert_eq!(state(exhausted.id).await, JobState::Failed);

    // Der alte Worker kann nach der Recovery nichts mehr festschreiben.
    assert!(
        !db.complete_job(before_callback.id, "dead-worker", &json!({}))
            .await
            .unwrap()
    );

    // uncertain wird nie automatisch wieder aufgenommen.
    let next = claim(&db, "w").await;
    assert_eq!(next.id, before_callback.id);
    assert!(matches!(
        db.claim_job("w", 60, 1000).await.unwrap(),
        ClaimOutcome::Empty
    ));

    // Abbrechen: nur nicht laufende Jobs.
    assert!(!db.cancel_job(next.id).await.unwrap(), "laufender Job");
    assert!(db.cancel_job(in_callback.id).await.unwrap());
}

#[tokio::test]
async fn concurrent_workers_never_share_a_job_and_respect_the_limit() {
    let Some(url) = test_database_url() else {
        return;
    };
    let db = fresh_schema(&url, "akl_jobs_concurrency").await;
    for i in 0..20 {
        enqueue(&db, &format!("c{i}"), 0, 3).await;
    }
    let mut handles = Vec::new();
    for w in 0..8 {
        let db = db.clone();
        handles.push(tokio::spawn(async move {
            let mut got = Vec::new();
            while let ClaimOutcome::Claimed(job) =
                db.claim_job(&format!("w{w}"), 60, 12).await.unwrap()
            {
                got.push(job.id);
            }
            got
        }));
    }
    let mut all = Vec::new();
    for h in handles {
        all.extend(h.await.unwrap());
    }
    let unique: std::collections::HashSet<_> = all.iter().collect();
    assert_eq!(all.len(), unique.len(), "kein Job doppelt vergeben");
    assert_eq!(all.len(), 12, "globales Limit gilt auch bei Parallelität");
}
