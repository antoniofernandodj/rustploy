//! Left navigation sidebar mirroring the TUI sections.

use crate::model::{palette, SidebarItem};
use crate::{App, Message};
use iced::widget::{button, column, container, scrollable, text, Space};
use iced::{Element, Length};

pub fn view(app: &App) -> Element<'_, Message> {
    let mut col = column![section_label("HOME")].spacing(2);

    for item in SidebarItem::HOME {
        col = col.push(item_button(app, *item));
    }

    col = col.push(Space::with_height(Length::Fixed(8.0)));
    col = col.push(item_button(app, SidebarItem::Projects));
    col = col.push(Space::with_height(Length::Fixed(8.0)));

    col = col.push(section_label("SETTINGS"));
    for item in SidebarItem::SETTINGS {
        col = col.push(item_button(app, *item));
    }

    col = col.push(Space::with_height(Length::Fixed(8.0)));
    col = col.push(section_label("ACCOUNT"));
    col = col.push(item_button(app, SidebarItem::Account));

    container(scrollable(col.padding(6)).height(Length::Fill))
        .width(Length::Fixed(210.0))
        .height(Length::Fill)
        .style(container::rounded_box)
        .into()
}

fn section_label(s: &str) -> Element<'_, Message> {
    text(s.to_string()).size(11).color(palette::GRAY).into()
}

fn item_button(app: &App, item: SidebarItem) -> Element<'_, Message> {
    let active = app.sidebar == item;
    let label = if item == SidebarItem::Projects && !app.projects.is_empty() {
        format!("{}  ({})", item.label(), app.projects.len())
    } else {
        item.label().to_string()
    };
    button(text(label).size(13))
        .on_press(Message::Sidebar(item))
        .width(Length::Fill)
        .style(if active { button::primary } else { button::text })
        .padding([4, 8])
        .into()
}
