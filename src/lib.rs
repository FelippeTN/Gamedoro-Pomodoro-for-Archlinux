//! Gamedoro: domínio, casos de uso, portas e adapters, compartilhados pelos dois binários:
//! `gamedoro` (daemon + CLI, sem GTK) e `gamedoro-menu` (menu de controle GTK, só enquanto aberto).

pub mod adapters;
pub mod app;
pub mod daemon;
pub mod domain;
pub mod ports;
