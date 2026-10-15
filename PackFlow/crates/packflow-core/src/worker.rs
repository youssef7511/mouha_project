use crate::{
    error::{DomainError, Result},
    models::{Mutation, Query},
    service::Engine,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::mpsc::{self, SyncSender},
};

enum Task {
    Query(Query, SyncSender<Result<Value>>),
    Mutate(Box<Mutation>, SyncSender<Result<Value>>),
    Backup(PathBuf, SyncSender<Result<Value>>),
    Restore(PathBuf, SyncSender<Result<Value>>),
}
#[derive(Clone)]
pub struct DbWorker {
    sender: SyncSender<Task>,
}
impl DbWorker {
    pub fn start(path: PathBuf) -> Result<Self> {
        let (sender, receiver) = mpsc::sync_channel::<Task>(64);
        let (ready, wait) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("packflow-db".into())
            .spawn(move || {
                let mut engine = match Engine::open(&path) {
                    Ok(engine) => {
                        let _ = ready.send(Ok(()));
                        engine
                    }
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                for task in receiver {
                    match task {
                        Task::Backup(path, reply) => {
                            let _ = reply.send(engine.backup(&path));
                        }
                        Task::Restore(path, reply) => {
                            let _ = reply.send(engine.restore(&path));
                        }
                        Task::Query(query, reply) => {
                            let _ = reply.send(engine.query(query));
                        }
                        Task::Mutate(mutation, reply) => {
                            let _ = reply.send(engine.mutate(*mutation));
                        }
                    }
                }
            })?;
        wait.recv()
            .map_err(|_| DomainError::storage("Database worker failed during startup."))??;
        Ok(Self { sender })
    }
    pub fn query(&self, query: Query) -> Result<Value> {
        let (reply, wait) = mpsc::sync_channel(1);
        self.sender
            .send(Task::Query(query, reply))
            .map_err(|_| DomainError::storage("Database worker stopped."))?;
        wait.recv()
            .map_err(|_| DomainError::storage("Database worker stopped."))?
    }
    pub fn backup(&self, path: PathBuf) -> Result<Value> {
        let (reply, wait) = mpsc::sync_channel(1);
        self.sender
            .send(Task::Backup(path, reply))
            .map_err(|_| DomainError::storage("Database worker stopped."))?;
        wait.recv()
            .map_err(|_| DomainError::storage("Database worker stopped."))?
    }
    pub fn restore(&self, path: PathBuf) -> Result<Value> {
        let (reply, wait) = mpsc::sync_channel(1);
        self.sender
            .send(Task::Restore(path, reply))
            .map_err(|_| DomainError::storage("Database worker stopped."))?;
        wait.recv()
            .map_err(|_| DomainError::storage("Database worker stopped."))?
    }
    pub fn mutate(&self, request: Mutation) -> Result<Value> {
        let (reply, wait) = mpsc::sync_channel(1);
        self.sender
            .send(Task::Mutate(Box::new(request), reply))
            .map_err(|_| DomainError::storage("Database worker stopped."))?;
        wait.recv()
            .map_err(|_| DomainError::storage("Database worker stopped."))?
    }
}
