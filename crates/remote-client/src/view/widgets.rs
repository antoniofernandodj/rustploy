//! Small reusable view helpers shared across screens.

use crate::model::palette;
use crate::Message;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};

pub fn section(s: &str) -> Element<'static, Message> {
    text(format!("── {s} ")).size(13).color(palette::YELLOW).into()
}

pub fn muted(s: impl text::IntoFragment<'static>) -> Element<'static, Message> {
    text(s).size(12).color(palette::GRAY).into()
}

pub fn label_text(s: &str) -> Element<'static, Message> {
    text(s.to_string()).size(13).color(palette::GRAY).into()
}

/// A labeled single-line input row (label fixed width, then the field).
pub fn labeled_input<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        container(text(label.to_string()).size(13).color(palette::GRAY)).width(Length::Fixed(190.0)),
        text_input(placeholder, value).on_input(on_input).padding(6).size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub fn primary_btn(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(14))
        .on_press(msg)
        .style(button::primary)
        .padding([10, 20])
        .into()
}

pub fn success_btn(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(14))
        .on_press(msg)
        .style(button::success)
        .padding([10, 20])
        .into()
}

pub fn danger_btn(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(14))
        .on_press(msg)
        .style(button::danger)
        .padding([10, 20])
        .into()
}

pub fn ghost_btn(label: &str, msg: Message) -> Element<'static, Message> {
    button(text(label.to_string()).size(14))
        .on_press(msg)
        .style(button::secondary)
        .padding([10, 20])
        .into()
}

/// A bordered content panel with a title, filling available space.
pub fn panel<'a>(title: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    container(
        column![
            text(title.to_string()).size(20).color(palette::CYAN),
            Space::with_height(Length::Fixed(18.0)),
            body,
        ]
        .spacing(2)
        .height(Length::Fill),
    )
    .padding(28)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(container::rounded_box)
    .into()
}

/// Renders a horizontal tab bar; `active` highlighted.
pub fn tab_bar<T: Copy + PartialEq>(
    tabs: &[(T, &str)],
    active: T,
    on_select: impl Fn(T) -> Message,
) -> Element<'static, Message> {
    let mut r = row![].spacing(8);
    for (tab, lbl) in tabs {
        let style = if *tab == active {
            button::primary
        } else {
            button::text
        };
        r = r.push(
            button(
                text((*lbl).to_string())
                    .size(14)
                    .wrapping(text::Wrapping::None),
            )
            .on_press(on_select(*tab))
            .style(style)
            .padding([8, 16]),
        );
    }
    // Em janelas estreitas a fileira de abas pode passar da largura — rola na
    // horizontal em vez de quebrar os rótulos verticalmente.
    scrollable(r)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill)
        .into()
}

pub fn placeholder<'a>(title: &'a str, desc: &'a str) -> Element<'a, Message> {
    panel(
        title,
        column![
            Space::with_height(Length::Fixed(8.0)),
            muted(desc.to_string()),
            Space::with_height(Length::Fixed(8.0)),
            text("Em construção.").size(13).color(palette::YELLOW),
        ]
        .spacing(4)
        .into(),
    )
}

