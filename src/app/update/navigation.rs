//! Selection / viewport navigation and confirm helpers.

use crate::app::action::{ConfirmChoice, Screen};
use crate::app::state::{AppState, Dialog};
use crate::app::update::side_effect::SideEffect;
use crate::model::{
    filter_services, is_protected_pid, ProcessInfo, ProcessSignal, ServiceActionKind, ServiceFilter,
};

pub(crate) fn list_len(state: &AppState) -> usize {
    if state.glossary_open() {
        return crate::glossary::filtered_terms(state.screen, state.current_search()).len();
    }
    match state.screen {
        Screen::Processes => {
            if state.process.tree_mode && !state.process.ppids.is_empty() {
                crate::model::build_process_tree(&state.process.items, &state.process.ppids).len()
            } else {
                state.visible_processes().len()
            }
        }
        Screen::Services => {
            let filter = if state.service.failed_only {
                ServiceFilter::Failed
            } else {
                state.service.filter
            };
            filter_services(&state.service.items, state.current_search(), filter).len()
        }
        Screen::Logs => state
            .log
            .buffer
            .filtered_preset(
                state.current_search(),
                state.log.min_priority,
                state.log.preset,
                state.log.unit.as_deref(),
            )
            .len(),
        Screen::Storage => match state.storage.tab {
            crate::model::StorageTab::DirectoryUsage => state.visible_storage_children().len(),
            crate::model::StorageTab::Mounts => state.metrics.disks.len(),
            crate::model::StorageTab::LargestFiles => state
                .storage
                .tree
                .as_ref()
                .map(|t| crate::preview::largest_files_from_tree(&t.root, 50).len())
                .unwrap_or(0),
        },
        Screen::Diagnostics => state.visible_findings().len(),
        Screen::Dashboard | Screen::Settings => 0,
    }
}

pub(crate) fn visible_rows(state: &AppState) -> usize {
    state.viewport_rows.max(1)
}

pub(crate) fn move_selection(state: &mut AppState, delta: i32) {
    if let Some(Dialog::ActionMenu {
        items, selected, ..
    }) = state.dialog.as_mut()
    {
        if items.is_empty() {
            return;
        }
        if delta < 0 {
            *selected = selected.saturating_sub(1);
        } else {
            *selected = (*selected + 1).min(items.len() - 1);
        }
        return;
    }
    if state.screen == Screen::Settings && !state.glossary_open() {
        let len = crate::settings::SETTINGS_SECTIONS.len();
        if len == 0 {
            return;
        }
        if delta < 0 {
            state.settings.section = state.settings.section.saturating_sub(1);
        } else {
            state.settings.section = (state.settings.section + 1).min(len - 1);
        }
        return;
    }
    let len = list_len(state);
    if len == 0 {
        return;
    }
    let vis = visible_rows(state);
    let vp = current_vp_mut(state);
    if delta <= -10 {
        vp.page_up(len, vis);
    } else if delta >= 10 {
        vp.page_down(len, vis);
    } else if delta < 0 {
        for _ in 0..((-delta) as usize) {
            vp.move_up(len, vis);
        }
    } else {
        for _ in 0..(delta as usize) {
            vp.move_down(len, vis);
        }
    }
    sync_selection_identity(state);
}

pub(crate) fn current_vp_mut(state: &mut AppState) -> &mut crate::viewport::ViewportState {
    if state.glossary_open() {
        return &mut state.glossary_vp;
    }
    match state.screen {
        Screen::Processes => &mut state.process.vp,
        Screen::Services => &mut state.service.vp,
        Screen::Logs => &mut state.log.vp,
        Screen::Storage => &mut state.storage.vp,
        Screen::Diagnostics => &mut state.diagnostic.vp,
        Screen::Dashboard | Screen::Settings => &mut state.process.vp, // unused
    }
}

pub(crate) fn sync_selection_identity(state: &mut AppState) {
    match state.screen {
        Screen::Processes => {
            let idx = state.process.vp.selected;
            let pid = if state.process.tree_mode && !state.process.ppids.is_empty() {
                crate::model::build_process_tree(&state.process.items, &state.process.ppids)
                    .get(idx)
                    .map(|(p, _, _)| *p)
            } else {
                state.visible_processes().get(idx).map(|p| p.pid)
            };
            if let Some(pid) = pid {
                state.process.selected_pid = Some(pid);
            }
        }
        Screen::Services => {
            let idx = state.service.vp.selected;
            let filter = if state.service.failed_only {
                ServiceFilter::Failed
            } else {
                state.service.filter
            };
            let filtered = filter_services(&state.service.items, state.current_search(), filter);
            if let Some(s) = filtered.get(idx) {
                state.service.selected_unit = Some(s.unit.clone());
            }
        }
        _ => {}
    }
}

pub(crate) fn set_selection(state: &mut AppState, idx: usize) {
    let len = list_len(state);
    let vis = visible_rows(state);
    let vp = current_vp_mut(state);
    vp.set_selected(idx, len, vis);
    sync_selection_identity(state);
}

pub(crate) fn clamp_selection_to_visible(state: &mut AppState) {
    let len = list_len(state);
    let vis = visible_rows(state);
    let vp = current_vp_mut(state);
    vp.clamp_to_len(len);
    vp.ensure_visible(len, vis);
    sync_selection_identity(state);
}

pub(crate) fn selected_service_unit(state: &AppState) -> Option<String> {
    let filter = if state.service.failed_only {
        ServiceFilter::Failed
    } else {
        state.service.filter
    };
    let filtered = filter_services(&state.service.items, state.current_search(), filter);
    filtered
        .get(state.service_selected())
        .map(|s| s.unit.clone())
}

pub(crate) fn selected_process_info(state: &AppState) -> Option<ProcessInfo> {
    if let Some(pid) = state.process.selected_pid {
        if let Some(p) = state.process.items.iter().find(|p| p.pid == pid) {
            return Some(p.clone());
        }
    }
    state
        .visible_processes()
        .get(state.process_selected())
        .map(|p| (*p).clone())
}

pub(crate) fn maybe_confirm_signal(
    state: &mut AppState,
    signal: ProcessSignal,
) -> Option<SideEffect> {
    if state.read_only {
        state.set_status("READ ONLY: signals disabled");
        return None;
    }
    let proc_ = selected_process_info(state)?;
    if is_protected_pid(proc_.pid, std::process::id()) {
        state.set_status(format!(
            "refusing {}: protected PID {}",
            signal.label(),
            proc_.pid
        ));
        return None;
    }
    let need_confirm = match signal {
        ProcessSignal::Term => state.config.confirm_sigterm,
        ProcessSignal::Kill => state.config.confirm_sigkill,
        ProcessSignal::Stop | ProcessSignal::Cont => true,
    };
    if !need_confirm {
        return Some(SideEffect::SendSignal {
            pid: proc_.pid,
            signal,
            start_time: proc_.start_time,
        });
    }
    state.dialog = Some(Dialog::ConfirmSignal {
        pid: proc_.pid,
        user: proc_.user.clone(),
        command: proc_.name.clone(),
        signal,
        start_time: proc_.start_time,
        choice: ConfirmChoice::Cancel,
    });
    None
}

pub(crate) fn maybe_confirm_service(
    state: &mut AppState,
    action: ServiceActionKind,
) -> Option<SideEffect> {
    if state.read_only {
        state.set_status("READ ONLY: service actions disabled");
        return None;
    }
    let unit = selected_service_unit(state)?;
    if !state.config.confirm_service_actions {
        return Some(SideEffect::ServiceAction { unit, action });
    }
    state.dialog = Some(Dialog::ConfirmService {
        unit,
        action,
        choice: ConfirmChoice::Cancel,
    });
    None
}

pub(crate) fn enter_storage_dir(state: &mut AppState) {
    let name = {
        let children = state.visible_storage_children();
        let child = match children.get(state.storage_selected()) {
            Some(c) => c,
            None => return,
        };
        if !child.is_dir {
            return;
        }
        child.name.clone()
    };
    state.storage.cwd.push(name);
    state.storage.vp.selected = 0;
}

pub(crate) fn step_log_match(state: &mut AppState, forward: bool) {
    let q = state.current_search().to_string();
    if q.is_empty() {
        return;
    }
    let filtered = state.log.buffer.filtered(&q, state.log.min_priority);
    if filtered.is_empty() {
        return;
    }
    let cur = state.log.match_idx.unwrap_or(state.log_selected());
    let next = if forward {
        (cur + 1) % filtered.len()
    } else if cur == 0 {
        filtered.len() - 1
    } else {
        cur - 1
    };
    state.log.match_idx = Some(next);
    state.log.vp.selected = next;
}
