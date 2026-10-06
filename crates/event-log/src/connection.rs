/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

use std::{future::Future, panic::AssertUnwindSafe, pin::Pin, sync::Arc};

mod authority;
pub use authority::ConnectionAuthority;

use futures_util::{FutureExt, future::Shared};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use tokio::sync::{Semaphore, mpsc, oneshot};

use crate::StoreError;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

type Job = Box<
    dyn for<'c> FnOnce(&'c mut SqliteConnection, Result<(), StoreError>) -> BoxFuture<'c, ()>
        + Send,
>;
type ReadJob = Box<
    dyn FnOnce(Result<SqliteConnection, StoreError>) -> BoxFuture<'static, Result<(), StoreError>>
        + Send,
>;
enum Command {
    Run(Job),
    Read(ReadJob),
    Shutdown,
}

/// Ingress is bounded; accepted work and the database lease belong to a dedicated
/// thread, independent of the caller's executor. Dropping ingress drains work.
pub struct OwnedConnection {
    jobs: mpsc::Sender<Command>,
    closed: Shared<BoxFuture<'static, Result<(), String>>>,
    readers: Arc<Semaphore>,
}

impl OwnedConnection {
    /// Observe the original worker's actual terminal result without closing ingress.
    pub(crate) fn close_receipt(&self) -> Shared<BoxFuture<'static, Result<(), String>>> {
        self.closed.clone()
    }

    pub async fn open(
        options: SqliteConnectOptions,
        authority: ConnectionAuthority,
        initialize: for<'c> fn(&'c mut SqliteConnection) -> BoxFuture<'c, Result<(), StoreError>>,
    ) -> Result<Self, StoreError> {
        let (jobs, mut receiver) = mpsc::channel::<Command>(32);
        let authority = Arc::new(authority);
        let (ready, startup) = oneshot::channel();
        let (completion, closed) = oneshot::channel();
        std::thread::Builder::new()
            .name("runtime-store-owner".into())
            .spawn(move || {
                let mut result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?;
                    let validate = || authority.validate();
                    validate()?;
                    let mut connection = runtime.block_on(connect_owned(&options))?;
                    let read_options = options.clone().create_if_missing(false).read_only(true);
                    let mut readers = tokio::task::JoinSet::new();
                    let mut reader_failure = None;
                    let initialized = std::panic::catch_unwind(AssertUnwindSafe(|| {
                        runtime.block_on(async {
                            initialize(&mut connection).await?;
                            validate()
                        })
                    }))
                    .unwrap_or(Err(StoreError::OperationUnknown));
                    let success = initialized.is_ok();
                    // If open's waiter disappeared, close without accepting work.
                    if ready.send(initialized).is_ok() && success {
                        while let Some(command) = runtime.block_on(receiver.recv()) {
                            while let Some(result) = readers.try_join_next() {
                                retain_reader_failure(result, &mut reader_failure);
                            }
                            let job = match command {
                                Command::Run(job) => job,
                                Command::Read(job) => {
                                    let options = read_options.clone();
                                    let valid = validate();
                                    let authority = authority.clone();
                                    readers.spawn_on(
                                        async move {
                                            let connection = match valid {
                                                Ok(()) => connect_owned(&options).await,
                                                Err(error) => Err(error),
                                            };
                                            let connection = match connection {
                                                Ok(connection) => match authority.validate() {
                                                    Ok(()) => Ok(connection),
                                                    Err(error) => {
                                                        Err(close_startup(connection, error).await)
                                                    }
                                                },
                                                Err(error) => Err(error),
                                            };
                                            let startup_cleanup = connection
                                                .as_ref()
                                                .err()
                                                .filter(|error| {
                                                    matches!(
                                                        error,
                                                        StoreError::StartupCleanupFailed { .. }
                                                    )
                                                })
                                                .map(ToString::to_string);
                                            let closed = job(connection).await;
                                            match startup_cleanup {
                                                Some(error) => Err(StoreError::CloseFailed(error)),
                                                None => closed,
                                            }
                                        },
                                        runtime.handle(),
                                    );
                                    continue;
                                }
                                Command::Shutdown => {
                                    // Reject new sends, then drain every accepted job.
                                    receiver.close();
                                    continue;
                                }
                            };
                            let executed = std::panic::catch_unwind(AssertUnwindSafe(|| {
                                runtime.block_on(job(&mut connection, validate()))
                            }));
                            if executed.is_err() {
                                // An unwound connection cannot accept further jobs.
                                break;
                            }
                        }
                    }
                    // Readers share this owner's authority, not the caller's lifetime.
                    // Close them before releasing the writer lease or acknowledging shutdown.
                    runtime.block_on(async {
                        while let Some(result) = readers.join_next().await {
                            retain_reader_failure(result, &mut reader_failure);
                        }
                    });
                    // Await real close before releasing authority, even after failure.
                    runtime
                        .block_on(connection.close())
                        .map_err(|error| StoreError::CloseFailed(error.to_string()))?;
                    match reader_failure {
                        Some(error) => Err(StoreError::CloseFailed(error)),
                        None => Ok(()),
                    }
                }))
                .unwrap_or_else(|_| {
                    Err(StoreError::StartupCleanupFailed {
                        cause: Box::new(StoreError::Initialization),
                        cleanup: Box::new(StoreError::OperationUnknown),
                    })
                });
                // Unexecuted accepted jobs still own their original payloads.
                // Their destructors cannot unwind past this authority boundary.
                if std::panic::catch_unwind(AssertUnwindSafe(|| drop(receiver))).is_err() {
                    result = Err(StoreError::CloseFailed(format!(
                        "queued connection work destruction panicked; prior close: {result:?}"
                    )));
                }
                let cleanup_failed = matches!(
                    result,
                    Err(StoreError::CloseFailed(_) | StoreError::StartupCleanupFailed { .. })
                );
                if !cleanup_failed {
                    drop(authority);
                }
                let _ = completion.send(result);
                // A failed real close is not permission to release the original
                // authority. Keep its actual worker alive; no replacement owner
                // or process exit can manufacture a cleanup acknowledgement.
                if cleanup_failed {
                    loop {
                        std::thread::park();
                    }
                }
            })?;
        match startup.await {
            Ok(Ok(())) => Ok(Self {
                jobs,
                closed: async move {
                    closed
                        .await
                        .map_err(|_| {
                            "connection owner exited without close acknowledgement".to_owned()
                        })?
                        .map_err(|error| error.to_string())
                }
                .boxed()
                .shared(),
                readers: Arc::new(Semaphore::new(2)),
            }),
            Ok(Err(error)) => {
                drop(jobs);
                match closed.await {
                    Ok(Ok(())) => Err(error),
                    result => Err(StoreError::StartupCleanupFailed {
                        cause: Box::new(error),
                        cleanup: Box::new(match result {
                            Ok(Err(error)) => error,
                            Err(_) => StoreError::OperationUnknown,
                            Ok(Ok(())) => unreachable!(),
                        }),
                    }),
                }
            }
            Err(_) => {
                drop(jobs);
                match closed.await {
                    Ok(Ok(())) => Err(StoreError::Initialization),
                    Ok(Err(error @ StoreError::StartupCleanupFailed { .. })) => Err(error),
                    Ok(Err(error)) if !matches!(error, StoreError::CloseFailed(_)) => {
                        // Owned startup confirms any acquired connection has closed;
                        // a crashed worker or failed close takes the typed branch above.
                        Err(error)
                    }
                    result => Err(StoreError::StartupCleanupFailed {
                        cause: Box::new(StoreError::Initialization),
                        cleanup: Box::new(match result {
                            Ok(Err(error)) => error,
                            Err(_) => StoreError::OperationUnknown,
                            Ok(Ok(())) => unreachable!(),
                        }),
                    }),
                }
            }
        }
    }

    pub async fn run<T: Send + 'static>(
        &self,
        operation: impl for<'c> FnOnce(&'c mut SqliteConnection) -> BoxFuture<'c, Result<T, StoreError>>
        + Send
        + 'static,
    ) -> Result<T, StoreError> {
        let (reply, result) = oneshot::channel();
        self.jobs
            .send(Command::Run(Box::new(move |connection, valid| {
                Box::pin(async move {
                    let result = match valid {
                        Ok(()) => operation(connection).await,
                        Err(error) => Err(error),
                    };
                    let _ = reply.send(result);
                })
            })))
            .await
            .map_err(|_| StoreError::ConnectionClosed)?;
        result.await.map_err(|_| StoreError::OperationUnknown)?
    }

    pub async fn shutdown(&self) -> Result<(), StoreError> {
        self.readers.close();
        // Closed ingress is also normal for repeated or concurrent shutdown.
        let _ = self.jobs.send(Command::Shutdown).await;
        self.closed.clone().await.map_err(StoreError::CloseFailed)
    }

    /// Bounded independent readers keep large snapshots off the serialized writer.
    /// Accepted reads drain on shutdown even if their caller or executor disappears.
    pub(crate) async fn read<T: Send + 'static>(
        &self,
        operation: impl for<'c> FnOnce(&'c mut SqliteConnection) -> BoxFuture<'c, Result<T, StoreError>>
        + Send
        + 'static,
    ) -> Result<T, StoreError> {
        let permit = self
            .readers
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| StoreError::ConnectionClosed)?;
        let (reply, result) = oneshot::channel();
        self.jobs
            .send(Command::Read(Box::new(move |connection| {
                Box::pin(async move {
                    let _permit = permit;
                    let (result, closed) = match connection {
                        Ok(mut connection) => {
                            let result =
                                AssertUnwindSafe(async { operation(&mut connection).await })
                                    .catch_unwind()
                                    .await
                                    .unwrap_or(Err(StoreError::OperationUnknown));
                            let closed = AssertUnwindSafe(connection.close())
                                .catch_unwind()
                                .await
                                .map_err(|_| {
                                    StoreError::CloseFailed("reader close panicked".into())
                                })
                                .and_then(|result| {
                                    result
                                        .map_err(|error| StoreError::CloseFailed(error.to_string()))
                                });
                            let result = match result {
                                Err(error) => Err(error),
                                Ok(value) => match &closed {
                                    Ok(()) => Ok(value),
                                    Err(error) => Err(StoreError::CloseFailed(error.to_string())),
                                },
                            };
                            (result, closed)
                        }
                        Err(error) => (Err(error), Ok(())),
                    };
                    let _ = reply.send(result);
                    closed
                })
            })))
            .await
            .map_err(|_| StoreError::ConnectionClosed)?;
        result.await.map_err(|_| StoreError::OperationUnknown)?
    }

    pub async fn close(self) -> Result<(), StoreError> {
        self.shutdown().await
    }
}

// These are the original owner's startup and reader cleanup boundaries, not
// another SQL owner. A returned ordinary startup error confirms actual close.
async fn close_startup(connection: SqliteConnection, cause: StoreError) -> StoreError {
    match connection.close().await {
        Ok(()) => cause,
        Err(cleanup) => StoreError::StartupCleanupFailed {
            cause: Box::new(cause),
            cleanup: Box::new(StoreError::CloseFailed(cleanup.to_string())),
        },
    }
}
async fn connect_owned(options: &SqliteConnectOptions) -> Result<SqliteConnection, StoreError> {
    let mut connection = match SqliteConnection::establish(options).await {
        Ok(connection) => connection,
        Err(sqlx::Error::WorkerCrashed) => {
            return Err(StoreError::StartupCleanupFailed {
                cause: Box::new(StoreError::Initialization),
                cleanup: Box::new(StoreError::OperationUnknown),
            });
        }
        Err(error) => return Err(error.into()),
    };
    if let Err(error) = connection.configure(options).await {
        return Err(close_startup(connection, error.into()).await);
    }
    Ok(connection)
}
fn retain_reader_failure(
    result: Result<Result<(), StoreError>, tokio::task::JoinError>,
    failure: &mut Option<String>,
) {
    if let Err(error) = result
        .map_err(|error| error.to_string())
        .and_then(|closed| closed.map_err(|error| error.to_string()))
    {
        failure.get_or_insert(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{File, OpenOptions},
        sync::{Arc, mpsc as blocking},
        time::Duration,
    };

    #[tokio::test]
    async fn failed_pragma_configuration_closes_the_established_worker_before_releasing_authority()
    {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("configured.sqlite");
        let lock = directory.path().join("writer.lock");
        let lease = Arc::new(crate::root::FileLease::acquire(&lock).unwrap());
        let original = Arc::downgrade(&lease);
        let result = OwnedConnection::open(
            SqliteConnectOptions::new()
                .filename(&database)
                .create_if_missing(true)
                .pragma("maka_invalid", "("),
            ConnectionAuthority::Writer { lease, root: None },
            |_| Box::pin(async { panic!("configuration must fail before initialization") }),
        )
        .await;
        assert!(matches!(
            result,
            Err(StoreError::Sqlx(sqlx::Error::Database(_)))
        ));
        assert!(
            database.exists(),
            "the actual SQLite handle must have been established"
        );
        assert!(original.upgrade().is_none());
        drop(crate::root::FileLease::acquire(&lock).unwrap());
    }

    #[tokio::test]
    async fn actual_reader_close_failure_fences_the_original_writer_and_root_authority() {
        struct PanickingDrop;
        impl Drop for PanickingDrop {
            fn drop(&mut self) {
                panic!("actual SQLite callback destructor failed");
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("root");
        let namespaces = crate::root::RootNamespaces {
            ownership: directory.path().join("owners"),
            control: directory.path().join("control"),
        };
        let authority = Arc::new(crate::root::RootOwner::create(&root, &namespaces).unwrap());
        let original = Arc::downgrade(&authority);
        let owner = OwnedConnection::open(
            SqliteConnectOptions::new()
                .filename(root.join(crate::root::ROOT_DATABASE))
                .create_if_missing(true),
            ConnectionAuthority::Root(authority),
            |_| Box::pin(async { Ok(()) }),
        )
        .await
        .unwrap();
        let read = owner
            .read(|connection| {
                Box::pin(async move {
                    let actual = PanickingDrop;
                    connection
                        .lock_handle()
                        .await?
                        .set_progress_handler(1, move || {
                            let _retained = &actual;
                            true
                        });
                    Ok(())
                })
            })
            .await;
        assert!(matches!(read, Err(StoreError::CloseFailed(_))));
        let (entered, reached) = oneshot::channel();
        let (release, gate) = oneshot::channel();
        owner
            .jobs
            .send(Command::Run(Box::new(move |_, _| {
                Box::pin(async move {
                    entered.send(()).unwrap();
                    gate.await.unwrap();
                    panic!("accepted writer operation unwound");
                })
            })))
            .await
            .unwrap();
        reached.await.unwrap();
        let queued_payload = PanickingDrop;
        owner
            .jobs
            .send(Command::Run(Box::new(move |_, _| {
                // The earlier operation unwinds before this queued job runs;
                // receiver destruction must settle its original payload here.
                drop(queued_payload);
                Box::pin(async {})
            })))
            .await
            .unwrap();
        release.send(()).unwrap();
        let closed = owner.shutdown().await;
        assert!(matches!(closed, Err(StoreError::CloseFailed(_))));
        assert!(
            closed
                .unwrap_err()
                .to_string()
                .contains("queued connection work destruction panicked")
        );
        drop(owner);
        assert!(
            original.upgrade().is_some(),
            "the same original SQL worker must retain authority"
        );
        assert_eq!(
            crate::root::RootOwner::open(&root, &namespaces)
                .err()
                .unwrap()
                .kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[tokio::test]
    async fn read_snapshot_does_not_block_writes_and_shutdown_drains_abandoned_readers() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("snapshot.sqlite");
        let lock = directory.path().join("writer.lock");
        let lease = Arc::new(crate::root::FileLease::acquire(&lock).unwrap());
        let contender = File::open(&lock).unwrap();
        let owner = Arc::new(OwnedConnection::open(
            SqliteConnectOptions::new().filename(&path).create_if_missing(true),
            ConnectionAuthority::Writer { lease, root: None },
            |connection| Box::pin(async move {
                sqlx::raw_sql("PRAGMA journal_mode=WAL; CREATE TABLE facts(value INTEGER); INSERT INTO facts VALUES(1);")
                    .execute(connection).await?;
                Ok(())
            }),
        ).await.unwrap());
        let (ready, opened) = oneshot::channel();
        let (release, gate) = oneshot::channel();
        let (finished, observed) = oneshot::channel();
        let reader = owner.clone();
        let caller = tokio::spawn(async move {
            reader
                .read(move |connection| {
                    Box::pin(async move {
                        let mut tx = connection.begin().await?;
                        let first: i64 = sqlx::query_scalar("SELECT SUM(value) FROM facts")
                            .fetch_one(&mut *tx)
                            .await?;
                        ready.send(()).unwrap();
                        gate.await.unwrap();
                        let second: i64 = sqlx::query_scalar("SELECT SUM(value) FROM facts")
                            .fetch_one(&mut *tx)
                            .await?;
                        assert!(
                            sqlx::query("INSERT INTO facts VALUES(99)")
                                .execute(&mut *tx)
                                .await
                                .is_err(),
                            "reader must not promote to a writer"
                        );
                        tx.rollback().await?;
                        finished.send((first, second)).unwrap();
                        Ok(())
                    })
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), opened)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(
            Duration::from_secs(5),
            owner.run(|connection| {
                Box::pin(async move {
                    sqlx::query("INSERT INTO facts VALUES(2)")
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            }),
        )
        .await
        .unwrap()
        .unwrap();
        caller.abort();
        let _ = caller.await;
        let closing = owner.clone();
        let shutdown = tokio::spawn(async move { closing.shutdown().await });
        assert!(
            tokio::time::timeout(Duration::from_millis(50), owner.closed.clone())
                .await
                .is_err()
        );
        assert!(matches!(
            contender.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        release.send(()).unwrap();
        assert_eq!(observed.await.unwrap(), (1, 1));
        tokio::time::timeout(Duration::from_secs(5), shutdown)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        contender.try_lock().unwrap();
        assert!(matches!(
            owner.read(|_| Box::pin(async { Ok(()) })).await,
            Err(StoreError::ConnectionClosed)
        ));
    }

    #[test]
    fn dropped_caller_runtime_keeps_authority_until_real_commit_and_close() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("owner.sqlite");
        let lock = directory.path().join("writer.lock");
        let lease = Arc::new(crate::root::FileLease::acquire(&lock).unwrap());
        let contender = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock)
            .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true);
        let owner = runtime
            .block_on(OwnedConnection::open(
                options,
                ConnectionAuthority::Writer { lease, root: None },
                |connection| {
                    Box::pin(async move {
                        sqlx::query("CREATE TABLE committed(value INTEGER)")
                            .execute(connection)
                            .await?;
                        Ok(())
                    })
                },
            ))
            .unwrap();
        let (entered, reached) = blocking::channel();
        let (release, gate) = blocking::channel();
        let close_ack = owner.closed.clone();
        runtime.spawn(async move {
            owner
                .run(move |connection| {
                    Box::pin(async move {
                        connection.lock_handle().await?.set_commit_hook(move || {
                            entered.send(()).unwrap();
                            gate.recv_timeout(Duration::from_secs(10)).unwrap();
                            true
                        });
                        sqlx::query("INSERT INTO committed VALUES (7)")
                            .execute(connection)
                            .await?;
                        Ok(())
                    })
                })
                .await
                .unwrap();
        });
        runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(5), async {
                while reached.try_recv().is_err() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
        });
        drop(runtime);
        assert!(matches!(
            contender.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        release.send(()).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            close_ack.await.unwrap();
            contender.try_lock().unwrap();
            let mut connection =
                SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&path))
                    .await
                    .unwrap();
            assert_eq!(
                sqlx::query_scalar::<_, i64>("SELECT value FROM committed")
                    .fetch_one(&mut connection)
                    .await
                    .unwrap(),
                7
            );
            connection.close().await.unwrap();
        });
    }

    #[tokio::test]
    async fn shutdown_is_idempotent_while_handles_remain_alive() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("writer.lock");
        let lease = Arc::new(crate::root::FileLease::acquire(&path).unwrap());
        let contender = File::open(&path).unwrap();
        let owner = Arc::new(
            OwnedConnection::open(
                SqliteConnectOptions::new().in_memory(true),
                ConnectionAuthority::Writer { lease, root: None },
                |_| Box::pin(async { Ok(()) }),
            )
            .await
            .unwrap(),
        );
        let retained = owner.clone();
        // Replacing the pathname must prevent the old worker from accepting new work.
        std::fs::rename(&path, directory.path().join("displaced.lock")).unwrap();
        let replacement = crate::root::FileLease::acquire(&path).unwrap();
        assert!(matches!(
            retained
                .run::<()>(|_| Box::pin(async { panic!("replaced lease must reject work") }))
                .await,
            Err(StoreError::Io(_))
        ));
        drop(replacement);
        let (first, second) = tokio::join!(owner.shutdown(), retained.shutdown());
        first.unwrap();
        second.unwrap();
        retained.shutdown().await.unwrap();
        contender.try_lock().unwrap();
        assert!(matches!(
            retained.run(|_| Box::pin(async { Ok(()) })).await,
            Err(StoreError::ConnectionClosed)
        ));
    }
}
