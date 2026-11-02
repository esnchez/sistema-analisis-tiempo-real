//! Puertos: traits que la aplicación necesita del exterior. Los implementa [`crate::infrastructure`].

mod event_publisher;
mod event_repository;

pub use event_publisher::{EventPublisher, PublishError, RawEvent};
pub use event_repository::{EventRepository, RepositoryError};
