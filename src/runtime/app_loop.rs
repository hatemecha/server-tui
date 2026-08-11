//! TUI event loop and provider pollers.

use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::app::event::AppEvent;
use crate::app::state::AppState;
use crate::app::update::apply_event;
use crate::config::Config;
use crate::error::AppError;
use crate::providers::ProviderBundle;
use crate::runtime::effects::handle_effects;
use crate::terminal::TerminalGuard;
use crate::ui;

pub async fn run_loop(
    terminal: &mut TerminalGuard,
    state: &mut AppState,
    providers: ProviderBundle,
    config: Config,
) -> Result<(), AppError> {
    let providers = Arc::new(providers);
    let (tx, mut rx) = mpsc::channel::<AppEvent>(256);
    let app_cancel = CancellationToken::new();
    let tracker = TaskTracker::new();
    let (config_tx, config_rx) = tokio::sync::watch::channel(config.clone());

    let size = terminal
        .terminal()
        .size()
        .map_err(|e| AppError::Terminal(e.to_string()))?;
    state.width = size.width;
    state.height = size.height;
    state.viewport_rows = crate::viewport::content_rows_from_terminal(state.height);

    spawn_pollers(
        Arc::clone(&providers),
        tx.clone(),
        app_cancel.clone(),
        config_rx,
        &tracker,
    );

    {
        let p = Arc::clone(&providers);
        let tx = tx.clone();
        tracker.spawn(async move {
            match p.metrics.collect().await {
                Ok(m) => {
                    let _ = tx.send(AppEvent::MetricsUpdated(m)).await;
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::Error(e)).await;
                }
            }
        });
    }

    let mut events = EventStream::new();
    let mut follow_cancel: Option<CancellationToken> = None;
    let mut scan_cancel: Option<CancellationToken> = None;

    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| AppError::Terminal(format!("SIGTERM handler: {e}")))?;
    let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
        .map_err(|e| AppError::Terminal(format!("SIGHUP handler: {e}")))?;

    loop {
        terminal
            .terminal()
            .draw(|f| ui::draw(f, state))
            .map_err(|e| AppError::Terminal(e.to_string()))?;

        if state.should_quit {
            break;
        }

        tokio::select! {
            _ = sigterm.recv() => {
                tracing::info!("received SIGTERM");
                state.should_quit = true;
            }
            _ = sighup.recv() => {
                tracing::info!("received SIGHUP");
                state.should_quit = true;
            }
            maybe_term = events.next() => {
                match maybe_term {
                    Some(Ok(Event::Key(key))) => {
                        if key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat {
                            let effects = apply_event(state, AppEvent::Input(key));
                            handle_effects(
                                effects,
                                state,
                                &providers,
                                &tx,
                                &app_cancel,
                                &mut follow_cancel,
                                &mut scan_cancel,
                                &config_tx,
                                &tracker,
                                terminal,
                            ).await;
                        }
                    }
                    Some(Ok(Event::Resize(w, h))) => {
                        let _ = apply_event(state, AppEvent::Resize(w, h));
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        tracing::warn!("input error: {e}");
                    }
                    None => break,
                }
            }
            maybe_ev = rx.recv() => {
                match maybe_ev {
                    Some(ev) => {
                        let effects = apply_event(state, ev);
                        handle_effects(
                            effects,
                            state,
                            &providers,
                            &tx,
                            &app_cancel,
                            &mut follow_cancel,
                            &mut scan_cancel,
                            &config_tx,
                            &tracker,
                            terminal,
                        ).await;
                    }
                    None => break,
                }
            }
        }
    }

    // Structured shutdown: cancel → wait children → timeout → abort.
    app_cancel.cancel();
    if let Some(c) = follow_cancel.take() {
        c.cancel();
    }
    if let Some(c) = scan_cancel.take() {
        c.cancel();
    }
    tracker.close();
    let shutdown = tokio::time::timeout(Duration::from_secs(2), tracker.wait());
    if shutdown.await.is_err() {
        tracing::warn!("task shutdown timed out; aborting remaining tasks");
        // TaskTracker wait timed out; remaining tasks drop on runtime shutdown.
    }
    Ok(())
}

fn spawn_pollers(
    providers: Arc<ProviderBundle>,
    tx: mpsc::Sender<AppEvent>,
    cancel: CancellationToken,
    config_rx: tokio::sync::watch::Receiver<Config>,
    tracker: &TaskTracker,
) {
    {
        let tx = tx.clone();
        let cancel = cancel.clone();
        let mut config_rx = config_rx.clone();
        tracker.spawn(async move {
            loop {
                let ms = config_rx.borrow().refresh_ms.max(200);
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    changed = config_rx.changed() => {
                        if changed.is_err() { break; }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {
                        let _ = tx.send(AppEvent::Tick).await;
                    }
                }
            }
        });
    }
    {
        let providers = Arc::clone(&providers);
        let tx = tx.clone();
        let cancel = cancel.clone();
        let mut config_rx = config_rx.clone();
        tracker.spawn(async move {
            loop {
                let ms = config_rx.borrow().refresh_ms.max(200);
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    changed = config_rx.changed() => {
                        if changed.is_err() { break; }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {
                        match providers.metrics.collect().await {
                            Ok(m) => { let _ = tx.send(AppEvent::MetricsUpdated(m)).await; }
                            Err(e) => { let _ = tx.send(AppEvent::Error(e)).await; }
                        }
                    }
                }
            }
        });
    }
    {
        let providers = Arc::clone(&providers);
        let tx = tx.clone();
        let cancel = cancel.clone();
        let mut config_rx = config_rx.clone();
        tracker.spawn(async move {
            loop {
                let ms = config_rx.borrow().process_refresh_ms.max(200);
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    changed = config_rx.changed() => {
                        if changed.is_err() { break; }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {
                        match providers.processes.list().await {
                            Ok(p) => { let _ = tx.send(AppEvent::ProcessesUpdated(p)).await; }
                            Err(e) => { let _ = tx.send(AppEvent::Error(e)).await; }
                        }
                    }
                }
            }
        });
    }
    {
        let providers = Arc::clone(&providers);
        let tx = tx.clone();
        let cancel = cancel.clone();
        let mut config_rx = config_rx.clone();
        tracker.spawn(async move {
            loop {
                let ms = config_rx.borrow().service_refresh_ms.max(200);
                tokio::select! {
                    _ = cancel.cancelled() => break,
                    changed = config_rx.changed() => {
                        if changed.is_err() { break; }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(ms)) => {
                        match providers.services.list_services().await {
                            Ok(s) => { let _ = tx.send(AppEvent::ServicesUpdated(s)).await; }
                            Err(e) => { let _ = tx.send(AppEvent::Error(e)).await; }
                        }
                    }
                }
            }
        });
    }
}
