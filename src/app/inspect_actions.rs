//! Inspect / action-menu / export handlers extracted from update.rs.

use crate::app::action::{AppAction, ConfirmChoice, Screen};
use crate::app::menus::{finding_menu, log_menu, process_menu, service_menu};
use crate::app::state::{AppState, Dialog};
use crate::app::update::SideEffect;
use crate::model::{filter_services, ExportFormat, MenuAction, ServiceFilter};
use crate::status::ToastKind;
use crate::support::{redact_cmd, SupportFormat};

fn visible_logs(state: &AppState) -> Vec<&crate::model::LogEntry> {
    state.log.buffer.filtered_preset(
        state.current_search(),
        state.log.min_priority,
        state.log.preset,
        state.log.unit.as_deref(),
    )
}

pub fn apply_inspect_action(state: &mut AppState, action: &AppAction) -> Option<Vec<SideEffect>> {
    match action {
        AppAction::Inspect => Some(do_inspect(state)),
        AppAction::OpenActionMenu => {
            open_menu(state);
            Some(Vec::new())
        }
        AppAction::ExportContext => Some(do_export(state)),
        AppAction::ActivateMenu | AppAction::MenuSelect => Some(activate_menu(state)),
        AppAction::ToggleProcessFollow => {
            if let Some(pid) = state.process.selected_pid {
                if state.process.follow_pid == Some(pid) {
                    state.process.follow_pid = None;
                    state.set_status("follow PID off");
                } else {
                    state.process.follow_pid = Some(pid);
                    state.set_success(format!("following PID {pid}"));
                }
            }
            Some(Vec::new())
        }
        AppAction::ToggleProcessTree => {
            state.process.tree_mode = !state.process.tree_mode;
            let mut effects = Vec::new();
            if state.process.tree_mode {
                effects.push(SideEffect::FetchPpidMap);
                state.set_status("tree view on (fetching parents)");
            } else {
                state.set_status("tree view off");
            }
            Some(effects)
        }
        AppAction::CompleteOnboarding => {
            state.dialog = Some(Dialog::ConfirmCompleteOnboarding {
                choice: ConfirmChoice::Cancel,
            });
            Some(Vec::new())
        }
        AppAction::SaveSettings => {
            state.config.terminal_profile = state.terminal_profile;
            state.config.performance_profile = state.performance_profile;
            state.config.wallboard_default = state.wallboard;
            match state.config.save_atomic(&state.config_path) {
                Ok(()) => {
                    state.settings.onboarding_pending = false;
                    state.set_success(format!(
                        "settings saved → {}",
                        crate::sanitize::sanitize_path_display(&state.config_path)
                    ));
                }
                Err(e) => state.set_error(e),
            }
            Some(vec![SideEffect::PublishConfig])
        }
        AppAction::CycleTerminalProfile => {
            state.terminal_profile = state.terminal_profile.next();
            state.config.terminal_profile = state.terminal_profile;
            state.set_status(format!(
                "terminal_profile: {}",
                state.terminal_profile.label()
            ));
            Some(Vec::new())
        }
        AppAction::CyclePerformanceProfile => {
            state.performance_profile = state.performance_profile.next();
            state.config.performance_profile = state.performance_profile;
            apply_performance(state);
            state.set_status(format!(
                "performance_profile: {}",
                state.performance_profile.label()
            ));
            Some(vec![SideEffect::PublishConfig])
        }
        AppAction::ToggleSetting(id) => {
            use crate::settings::SettingId;
            match id {
                SettingId::ConfirmSigterm => {
                    state.config.confirm_sigterm = !state.config.confirm_sigterm
                }
                SettingId::ConfirmSigkill => {
                    state.config.confirm_sigkill = !state.config.confirm_sigkill
                }
                SettingId::ConfirmServiceActions => {
                    state.config.confirm_service_actions = !state.config.confirm_service_actions
                }
                SettingId::EnableSmartProbes => {
                    state.config.enable_smart_probes = !state.config.enable_smart_probes
                }
                SettingId::DiagnosticsLightScan => {
                    state.config.diagnostics_light_scan = !state.config.diagnostics_light_scan
                }
                SettingId::Wallboard => {
                    state.wallboard = !state.wallboard;
                    state.config.wallboard_default = state.wallboard;
                }
                SettingId::Color => {
                    state.color = !state.color;
                    state.config.color = state.color;
                }
            }
            state.set_status(format!("toggled {} (Save with S)", id.label()));
            Some(Vec::new())
        }
        AppAction::ResetSettings => {
            state.dialog = Some(Dialog::ConfirmResetSettings {
                choice: ConfirmChoice::Cancel,
            });
            Some(Vec::new())
        }
        _ => None,
    }
}

fn apply_performance(state: &mut AppState) {
    let scale = state.performance_profile.refresh_scale();
    let base = 1000u64;
    state.config.refresh_ms = ((base as f64) * scale) as u64;
    state.config.metric_history_size = state
        .performance_profile
        .history_size(state.config.metric_history_size);
}

fn do_inspect(state: &mut AppState) -> Vec<SideEffect> {
    let mut effects = Vec::new();
    match state.screen {
        Screen::Processes => {
            if let Some(pid) = state.process.selected_pid {
                effects.push(SideEffect::FetchProcessDetails { pid });
                state.set_toast(
                    ToastKind::Progress,
                    format!("loading details for PID {pid}"),
                );
            }
        }
        Screen::Services => {
            if let Some(unit) = state.service.selected_unit.clone() {
                state.service.pending_inspect = true;
                effects.push(SideEffect::FetchServiceDetails { unit: unit.clone() });
                effects.push(SideEffect::RefreshLogs { unit: Some(unit) });
                state.set_toast(ToastKind::Progress, "loading service details");
            }
        }
        Screen::Logs => {
            let filtered = visible_logs(state);
            if let Some(e) = filtered.get(state.log_selected()).copied() {
                // Use full buffer indices when possible.
                let all: Vec<_> = state.log.buffer.iter().collect();
                let idx = all.iter().position(|x| std::ptr::eq(*x, e)).unwrap_or(0);
                if let Some((before, focus, after)) = state.log.buffer.context_around(idx, 5, 5) {
                    let ctx = crate::model::LogInspectContext {
                        index: idx,
                        before,
                        focus,
                        after,
                    };
                    state.dialog = Some(Dialog::Inspector {
                        title: "Log event".into(),
                        body: ctx.format_body(),
                    });
                }
            }
        }
        Screen::Storage => match state.storage.tab {
            crate::model::StorageTab::DirectoryUsage => {
                if let Some(node) = state
                    .visible_storage_children()
                    .get(state.storage_selected())
                    .copied()
                {
                    if node.is_dir {
                        // EnterDir already handles; show node details
                        state.dialog = Some(Dialog::Inspector {
                            title: format!("Dir {}", node.name),
                            body: format!(
                                "path: {}\nsize: {}\napparent: {}\nchildren: {}",
                                node.path.display(),
                                crate::model::format_size(node.size),
                                crate::model::format_size(node.apparent_size),
                                node.children.len()
                            ),
                        });
                    } else {
                        effects.push(SideEffect::PreviewFile {
                            path: node.path.clone(),
                        });
                        state.set_toast(ToastKind::Progress, "previewing file");
                    }
                }
            }
            crate::model::StorageTab::LargestFiles => {
                if let Some(tree) = state.storage.tree.as_ref() {
                    let files = crate::preview::largest_files_from_tree(&tree.root, 50);
                    if let Some(node) = files.get(state.storage_selected()).copied() {
                        effects.push(SideEffect::PreviewFile {
                            path: node.path.clone(),
                        });
                    }
                }
            }
            crate::model::StorageTab::Mounts => {
                if let Some(d) = state.metrics.disks.get(state.storage_selected()) {
                    let pct = if d.total == 0 {
                        0.0
                    } else {
                        d.used as f64 / d.total as f64 * 100.0
                    };
                    state.dialog = Some(Dialog::Inspector {
                        title: format!("Mount {}", d.mount_point),
                        body: format!(
                            "mount: {}\ndevice: {}\nused: {} / {} ({:.0}%)\navailable: {}",
                            d.mount_point,
                            d.name,
                            crate::model::format_bytes(d.used),
                            crate::model::format_bytes(d.total),
                            pct,
                            crate::model::format_bytes(d.available)
                        ),
                    });
                }
            }
        },
        Screen::Diagnostics => {
            if let Some(f) = state
                .visible_findings()
                .get(state.finding_selected())
                .copied()
            {
                state.dialog = Some(Dialog::Inspector {
                    title: f.title.clone(),
                    body: format!(
                        "{}\n\n{}\n\nevidence:\n{}\n{}",
                        f.summary,
                        f.id,
                        f.evidence.summary,
                        f.evidence.details.join("\n")
                    ),
                });
            }
        }
        Screen::Dashboard => {
            state.screen = Screen::Diagnostics;
            effects.push(SideEffect::RefreshDiagnostics);
        }
        Screen::Settings => {}
    }
    effects
}

fn open_menu(state: &mut AppState) {
    let (title, items) = match state.screen {
        Screen::Processes => ("Process actions".into(), process_menu(state)),
        Screen::Services => {
            let filter = if state.service.failed_only {
                ServiceFilter::Failed
            } else {
                state.service.filter
            };
            let filtered = filter_services(&state.service.items, state.current_search(), filter);
            let (active, ufs) = filtered
                .get(state.service_selected())
                .map(|s| (s.active_state.as_str(), s.unit_file_state))
                .unwrap_or(("unknown", crate::model::UnitFileState::Unknown));
            ("Service actions".into(), service_menu(state, active, ufs))
        }
        Screen::Logs => ("Log actions".into(), log_menu()),
        Screen::Diagnostics => ("Finding actions".into(), finding_menu()),
        _ => (
            "Actions".into(),
            vec![crate::model::MenuItem::enabled("Close", MenuAction::Close)],
        ),
    };
    state.dialog = Some(Dialog::ActionMenu {
        title,
        items,
        selected: 0,
    });
}

fn activate_menu(state: &mut AppState) -> Vec<SideEffect> {
    let Some(Dialog::ActionMenu {
        items, selected, ..
    }) = state.dialog.clone()
    else {
        return Vec::new();
    };
    let Some(item) = items.get(selected) else {
        return Vec::new();
    };
    if !item.enabled {
        state.set_warning(
            item.disabled_reason
                .clone()
                .unwrap_or_else(|| "action disabled".into()),
        );
        return Vec::new();
    }
    state.dialog = None;
    match item.action {
        MenuAction::Close => Vec::new(),
        MenuAction::ProcessInspect | MenuAction::ServiceInspect => do_inspect(state),
        MenuAction::ProcessFollow => {
            let _ = apply_inspect_action(state, &AppAction::ToggleProcessFollow);
            Vec::new()
        }
        MenuAction::ProcessTree => {
            apply_inspect_action(state, &AppAction::ToggleProcessTree).unwrap_or_default()
        }
        MenuAction::ProcessLogs => {
            if let Some(unit) = state.process.details.as_ref().and_then(|d| d.unit.clone()) {
                state.log.unit = Some(unit.clone());
                state.screen = Screen::Logs;
                vec![SideEffect::RefreshLogs { unit: Some(unit) }]
            } else {
                state.set_warning("no unit associated with process");
                Vec::new()
            }
        }
        MenuAction::ProcessService => {
            if let Some(unit) = state.process.details.as_ref().and_then(|d| d.unit.clone()) {
                state.screen = Screen::Services;
                if let Some(slot) = state.search.get_mut(Screen::Services) {
                    *slot = unit;
                }
            }
            Vec::new()
        }
        MenuAction::ProcessDiagnostics | MenuAction::ServiceDiagnostics => {
            state.screen = Screen::Diagnostics;
            vec![SideEffect::RefreshDiagnostics]
        }
        MenuAction::ProcessExport => export_process(state),
        MenuAction::ServiceExport => export_service(state),
        MenuAction::ServiceLogs => {
            if let Some(unit) = state.service.selected_unit.clone() {
                state.log.unit = Some(unit.clone());
                state.screen = Screen::Logs;
                vec![SideEffect::RefreshLogs { unit: Some(unit) }]
            } else {
                Vec::new()
            }
        }
        MenuAction::ServiceAction(kind) => {
            // Re-use confirmation path via synthetic - call maybe via dialog open is in update
            // Signal by setting a temporary approach: open confirm through existing helpers not accessible.
            // Use ExecuteService after confirm elsewhere — open ConfirmService here.
            if let Some(unit) = state.service.selected_unit.clone() {
                if state.read_only {
                    state.set_status("READ ONLY");
                    return Vec::new();
                }
                state.dialog = Some(Dialog::ConfirmService {
                    unit,
                    action: kind,
                    choice: ConfirmChoice::Cancel,
                });
            }
            Vec::new()
        }
        MenuAction::ProcessSignal(sig) => {
            // Open confirm by mimicking: use dialog directly
            if let Some(p) = state
                .visible_processes()
                .get(state.process_selected())
                .copied()
            {
                if state.read_only {
                    state.set_status("READ ONLY");
                    return Vec::new();
                }
                state.dialog = Some(Dialog::ConfirmSignal {
                    pid: p.pid,
                    user: p.user.clone(),
                    command: p.name.clone(),
                    signal: sig,
                    start_time: p.start_time,
                    choice: ConfirmChoice::Cancel,
                });
            }
            Vec::new()
        }
        MenuAction::LogExportSelected(fmt) => export_logs_selected(state, fmt),
        MenuAction::LogExportVisible(fmt) => export_logs_visible(state, fmt),
        MenuAction::LogExportContext(fmt) => export_logs_context(state, fmt),
        MenuAction::FindingExport => export_finding(state),
        MenuAction::StoragePreview => do_inspect(state),
    }
}

fn do_export(state: &mut AppState) -> Vec<SideEffect> {
    match state.screen {
        Screen::Processes => export_process(state),
        Screen::Services => export_service(state),
        Screen::Logs => export_logs_visible(state, ExportFormat::Text),
        Screen::Diagnostics => export_finding(state),
        _ => {
            state.set_status("nothing to export on this screen");
            Vec::new()
        }
    }
}

fn export_process(state: &mut AppState) -> Vec<SideEffect> {
    let Some(p) = state
        .visible_processes()
        .get(state.process_selected())
        .copied()
    else {
        return Vec::new();
    };
    let details = state.process.details.as_ref();
    let body = format!(
        "# process export (redacted)\npid: {}\nuser: {}\nname: {}\ncmd: {}\ncpu: {:.1}\nmem: {}\nstate: {}\n{}\n",
        p.pid,
        p.user,
        p.name,
        redact_cmd(&p.cmd, false),
        p.cpu,
        crate::model::format_mem_pct(p.mem_pct),
        p.state,
        details.map(|d| d.format_body()).unwrap_or_default()
    );
    vec![SideEffect::SaveReport {
        body,
        format: SupportFormat::Markdown,
    }]
}

fn export_service(state: &mut AppState) -> Vec<SideEffect> {
    let filter = if state.service.failed_only {
        ServiceFilter::Failed
    } else {
        state.service.filter
    };
    let filtered = filter_services(&state.service.items, state.current_search(), filter);
    let Some(s) = filtered.get(state.service_selected()) else {
        return Vec::new();
    };
    let mut body = format!(
        "# service export\nunit: {}\nactive: {} ({})\nenabled: {}\nfragment: {:?}\n\n## recent logs\n",
        s.unit,
        s.active_state,
        s.sub_state,
        s.unit_file_state.label(),
        s.fragment_path
    );
    for e in state.service.recent_logs.iter().take(10) {
        body.push_str(&format!(
            "- {} {} {}\n",
            e.timestamp,
            e.priority.label(),
            e.message
        ));
    }
    vec![SideEffect::SaveReport {
        body,
        format: SupportFormat::Markdown,
    }]
}

fn support_fmt(fmt: ExportFormat) -> SupportFormat {
    match fmt {
        ExportFormat::Text => SupportFormat::Text,
        ExportFormat::Markdown => SupportFormat::Markdown,
        ExportFormat::Json => SupportFormat::Json,
    }
}

fn export_logs_selected(state: &mut AppState, fmt: ExportFormat) -> Vec<SideEffect> {
    let filtered = visible_logs(state);
    let Some(e) = filtered.get(state.log_selected()).copied() else {
        return Vec::new();
    };
    let body = match fmt {
        ExportFormat::Json => serde_json::json!({
            "timestamp": e.timestamp,
            "priority": e.priority.label(),
            "unit": e.unit,
            "pid": e.pid,
            "message": e.message,
        })
        .to_string(),
        ExportFormat::Markdown => format!(
            "# log event\n- time: {}\n- priority: {}\n- unit: {}\n- pid: {:?}\n\n{}\n",
            e.timestamp,
            e.priority.label(),
            e.unit,
            e.pid,
            e.message
        ),
        ExportFormat::Text => format!(
            "{} {} {} {:?}\n{}\n",
            e.timestamp,
            e.priority.label(),
            e.unit,
            e.pid,
            e.message
        ),
    };
    vec![SideEffect::SaveReport {
        body,
        format: support_fmt(fmt),
    }]
}

fn export_logs_visible(state: &mut AppState, fmt: ExportFormat) -> Vec<SideEffect> {
    let filtered = visible_logs(state);
    let body = match fmt {
        ExportFormat::Json => {
            let rows: Vec<_> = filtered
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "timestamp": e.timestamp,
                        "priority": e.priority.label(),
                        "unit": e.unit,
                        "pid": e.pid,
                        "message": e.message,
                    })
                })
                .collect();
            serde_json::to_string_pretty(&rows).unwrap_or_else(|_| "[]".into())
        }
        ExportFormat::Markdown => {
            let mut body = String::from("# logs export\n\n");
            for e in &filtered {
                body.push_str(&format!(
                    "- `{}` **{}** {} — {}\n",
                    e.timestamp,
                    e.priority.label(),
                    e.unit,
                    e.message
                ));
            }
            body
        }
        ExportFormat::Text => {
            let mut body = String::new();
            for e in &filtered {
                body.push_str(&format!(
                    "{} {} {} {}\n",
                    e.timestamp,
                    e.priority.label(),
                    e.unit,
                    e.message
                ));
            }
            body
        }
    };
    vec![SideEffect::SaveReport {
        body,
        format: support_fmt(fmt),
    }]
}

fn export_logs_context(state: &mut AppState, fmt: ExportFormat) -> Vec<SideEffect> {
    let filtered = visible_logs(state);
    let Some(e) = filtered.get(state.log_selected()).copied() else {
        return Vec::new();
    };
    let all: Vec<_> = state.log.buffer.iter().collect();
    let idx = all.iter().position(|x| std::ptr::eq(*x, e)).unwrap_or(0);
    let Some((before, focus, after)) = state.log.buffer.context_around(idx, 5, 5) else {
        return Vec::new();
    };
    let ctx = crate::model::LogInspectContext {
        index: idx,
        before,
        focus,
        after,
    };
    let body = match fmt {
        ExportFormat::Json => serde_json::to_string_pretty(&serde_json::json!({
            "index": ctx.index,
            "before": ctx.before.iter().map(|e| e.message.clone()).collect::<Vec<_>>(),
            "focus": ctx.focus.message,
            "after": ctx.after.iter().map(|e| e.message.clone()).collect::<Vec<_>>(),
        }))
        .unwrap_or_else(|_| "{}".into()),
        _ => format!("# log context\n\n{}", ctx.format_body()),
    };
    vec![SideEffect::SaveReport {
        body,
        format: support_fmt(fmt),
    }]
}

fn export_finding(state: &mut AppState) -> Vec<SideEffect> {
    let Some(f) = state
        .visible_findings()
        .get(state.finding_selected())
        .copied()
    else {
        return Vec::new();
    };
    let body = format!(
        "# finding {}\nseverity: {}\n{}\n\n## evidence\n{}\n{}\n",
        f.id,
        f.severity.label(),
        f.summary,
        f.evidence.summary,
        f.evidence.details.join("\n")
    );
    vec![SideEffect::SaveReport {
        body,
        format: SupportFormat::Markdown,
    }]
}
