//! Dominio: tipos y reglas del negocio. No hace I/O ni depende de la infraestructura.

#[macro_use]
mod macros;

mod content;
mod context;
mod error;
pub mod event;
mod id;
mod value;

pub use content::{ContentRef, ContentType};
pub use context::{ClientContext, DeviceType, Platform, TrafficSource};
pub use error::DomainError;
pub use id::{ContentId, SessionId, UserId};
pub use value::{CountryCode, Currency, Money, Percent};
