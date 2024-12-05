use actix_web::web;
use std::{
    ffi::OsString,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub struct Limit {
    pub count: usize,
    pub max: usize,
}

impl Limit {
    pub fn new(max: usize) -> Self {
        Self { count: 0, max }
    }
}

impl Limit {
    pub fn is_reached(&self) -> bool {
        self.count >= self.max
    }

    pub fn increment(&mut self) {
        self.count += 1;
    }

    pub fn decrement(&mut self) {
        self.count -= 1;
    }
}

impl From<OsString> for Limit {
    fn from(s: OsString) -> Self {
        match s.into_string() {
            Ok(s) => match s.parse() {
                Ok(max) => Self::new(max),
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        }
    }
}

impl Default for Limit {
    fn default() -> Self {
        Self::new(15)
    }
}

impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.count, self.max)
    }
}

pub fn try_acquire_session(
    session_count: &web::Data<Arc<Mutex<Limit>>>,
) -> Result<(), actix_web::Error> {
    let mut session_count_guard = session_count.lock().map_err(|_| {
        log::error!("Failed to acquire session count lock");
        actix_web::error::ErrorInternalServerError("Failed to acquire session count lock")
    })?;

    if session_count_guard.is_reached() {
        log::warn!("Too many concurrent sessions; rejecting new session");
        return Err(actix_web::error::ErrorTooManyRequests(
            "Too many concurrent sessions",
        ));
    }

    session_count_guard.increment();
    log::debug!("Session started. Active sessions: {}", session_count_guard);
    Ok(())
}

pub fn release_session(session_count: &web::Data<Arc<Mutex<Limit>>>) {
    if let Ok(mut session_count_guard) = session_count.lock() {
        session_count_guard.decrement();
        log::debug!("Session ended. Active sessions: {}", session_count_guard);
    } else {
        log::error!("Failed to acquire session count lock for decrement");
    }
}
