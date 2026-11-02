//! Capa de aplicación: casos de uso (p. ej. procesar un evento de origen) y los puertos
//! que estos necesitan, como la lectura de eventos o su persistencia.
//!
//! Sigue CQS: los casos de uso que cambian estado son [`command`]s, que no devuelven datos.
//!
//! Depende solo de [`crate::domain`].

pub mod command;
pub mod port;
