//! PyO3 bindings for servo-fetch.

use std::collections::HashMap;
use std::path::PathBuf;

use pyo3::prelude::*;

mod client;
mod console;
mod crawl;
mod errors;
mod opts;
mod page;
mod schema;
mod session;
mod validate;

use crate::errors::map_error;
use crate::opts::{BuildOpts, prepare};

/// Fetch, render, and extract a single URL.
#[pyfunction]
#[pyo3(signature = (url, *, timeout=None, settle=None, user_agent=None, screenshot=false, full_page=true, javascript=None, schema=None, cookies_file=None, headers=None, network_policy=None, zoom=None))]
#[allow(clippy::too_many_arguments)]
fn fetch(
    py: Python<'_>,
    url: String,
    timeout: Option<f64>,
    settle: Option<f64>,
    user_agent: Option<String>,
    screenshot: bool,
    full_page: bool,
    javascript: Option<String>,
    schema: Option<Bound<'_, schema::Schema>>,
    cookies_file: Option<PathBuf>,
    headers: Option<HashMap<String, String>>,
    network_policy: Option<&str>,
    zoom: Option<f64>,
) -> PyResult<page::Page> {
    let policy = match network_policy {
        None => None,
        Some("strict") => Some(servo_fetch::NetworkPolicy::STRICT),
        Some("permissive") => Some(servo_fetch::NetworkPolicy::PERMISSIVE),
        Some("permissive_local") => Some(servo_fetch::NetworkPolicy::PERMISSIVE_LOCAL),
        Some(s) => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "invalid network_policy '{s}': must be 'strict', 'permissive', or 'permissive_local'"
            )));
        }
    };
    let prepared = prepare(BuildOpts {
        url,
        timeout,
        settle,
        user_agent,
        screenshot,
        full_page,
        javascript,
        schema,
        cookies_file,
        headers,
        network_policy: policy,
        zoom,
    })?;
    let servo_page = py
        .detach(|| servo_fetch::blocking::fetch(&prepared.opts))
        .map_err(map_error)?;
    Ok(page::Page::new(
        servo_page,
        prepared.url,
        prepared.screenshot_requested,
        prepared.js_requested,
    ))
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(fetch, m)?)?;
    m.add_class::<page::Page>()?;
    m.add_class::<schema::Schema>()?;
    m.add_class::<schema::Field>()?;
    m.add_class::<client::Client>()?;
    m.add_class::<session::Session>()?;
    m.add_function(wrap_pyfunction!(session::run_worker_stdio, m)?)?;
    m.add_class::<console::ConsoleMessage>()?;
    m.add_class::<crawl::CrawlResult>()?;
    m.add_class::<crawl::MappedUrl>()?;
    errors::register(py, m)?;
    Ok(())
}
