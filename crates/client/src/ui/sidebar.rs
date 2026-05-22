use crate::app::{App, Focus, SidebarItem};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Sidebar;
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let selectable = app.selectable_sidebar_items();
    let mut items: Vec<ListItem> = vec![];
    let mut sel_idx = 0usize;

    // HOME section
    items.push(make_header("HOME"));
    for item in &[
        SidebarItem::HomeDeployments,
        SidebarItem::HomeMonitoring,
        SidebarItem::HomeSchedules,
        SidebarItem::HomeIngress,
        SidebarItem::HomeDocker,
        SidebarItem::HomeDeployEngine,
        SidebarItem::HomeRequests,
    ] {
        let selected = focused && sel_idx == app.sidebar_cursor;
        items.push(make_item(item.label(&app.projects), selected));
        sel_idx += 1;
    }

    // PROJECTS section
    items.push(make_blank());
    items.push(make_header("PROJECTS"));

    let selected = focused && sel_idx == app.sidebar_cursor;
    items.push(make_item(SidebarItem::NewProject.label(&app.projects), selected));
    sel_idx += 1;

    for i in 0..app.projects.len() {
        let is_active = app.active_project_id.as_deref() == Some(&app.projects[i].id);
        let selected = focused && sel_idx == app.sidebar_cursor;

        let label = if is_active && !selected {
            format!("► {}", app.projects[i].name)
        } else {
            format!("  {}", app.projects[i].name)
        };

        let style = if selected {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else if is_active {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };

        items.push(ListItem::new(Line::from(Span::styled(label, style))));
        sel_idx += 1;
    }

    // SETTINGS section
    items.push(make_blank());
    items.push(make_header("SETTINGS"));
    for item in &[
        SidebarItem::SettingsWebServer,
        SidebarItem::SettingsProfile,
        SidebarItem::SettingsUsers,
        SidebarItem::SettingsAuditLogs,
        SidebarItem::SettingsSshKeys,
        SidebarItem::SettingsTags,
        SidebarItem::SettingsGit,
        SidebarItem::SettingsRegistry,
        SidebarItem::SettingsS3,
        SidebarItem::SettingsCerts,
        SidebarItem::SettingsSso,
    ] {
        let selected = focused && sel_idx == app.sidebar_cursor;
        items.push(make_item(item.label(&app.projects), selected));
        sel_idx += 1;
    }

    // ACCOUNT
    items.push(make_blank());
    let selected = focused && sel_idx == app.sidebar_cursor;
    items.push(make_item(SidebarItem::Account.label(&app.projects), selected));

    let sidebar_block =
        Block::default().borders(Borders::RIGHT).border_style(border_style);
    let list = List::new(items).block(sidebar_block);

    let mut state = ListState::default();
    if let Some(sel_item) = selectable.get(app.sidebar_cursor) {
        state.select(Some(compute_visual_index(app, sel_item)));
    }

    f.render_stateful_widget(list, area, &mut state);
}

fn make_header(label: &str) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        label.to_string(),
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )))
}

fn make_item(label: String, selected: bool) -> ListItem<'static> {
    let style = if selected {
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    ListItem::new(Line::from(Span::styled(label, style)))
}

fn make_blank() -> ListItem<'static> {
    ListItem::new(Line::from(""))
}

fn compute_visual_index(app: &App, target: &SidebarItem) -> usize {
    // Layout offsets:
    // 0: HOME header
    // 1-7: home items
    // 8: blank
    // 9: PROJECTS header
    // 10: New Project
    // 11..10+n: projects
    // 11+n: blank
    // 12+n: SETTINGS header
    // 13+n..23+n: settings items (11)
    // 24+n: blank
    // 25+n: ACCOUNT
    let n = app.projects.len();
    match target {
        SidebarItem::HomeDeployments => 1,
        SidebarItem::HomeMonitoring => 2,
        SidebarItem::HomeSchedules => 3,
        SidebarItem::HomeIngress => 4,
        SidebarItem::HomeDocker => 5,
        SidebarItem::HomeDeployEngine => 6,
        SidebarItem::HomeRequests => 7,
        SidebarItem::NewProject => 10,
        SidebarItem::Project(i) => 11 + i,
        SidebarItem::SettingsWebServer => 13 + n,
        SidebarItem::SettingsProfile => 14 + n,
        SidebarItem::SettingsUsers => 15 + n,
        SidebarItem::SettingsAuditLogs => 16 + n,
        SidebarItem::SettingsSshKeys => 17 + n,
        SidebarItem::SettingsTags => 18 + n,
        SidebarItem::SettingsGit => 19 + n,
        SidebarItem::SettingsRegistry => 20 + n,
        SidebarItem::SettingsS3 => 21 + n,
        SidebarItem::SettingsCerts => 22 + n,
        SidebarItem::SettingsSso => 23 + n,
        SidebarItem::Account => 25 + n,
    }
}
