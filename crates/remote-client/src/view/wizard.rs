//! New-service creation wizard: type picker, database picker, application /
//! database / compose forms, and the template catalog. Mirrors the TUI flow.

use super::widgets::*;
use crate::model::{palette, DbKind, NsField, NsForm, NsStep, ServiceKind};
use crate::{App, Message};
use iced::widget::{button, checkbox, column, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};
use shared::templates::{self, TemplateCategory};

pub fn view(app: &App) -> Element<'_, Message> {
    let Some(ns) = &app.ns else {
        return text("").into();
    };
    let body = match ns.step {
        NsStep::PickType => pick_type(),
        NsStep::PickDb => pick_db(),
        NsStep::AppForm => app_form(ns),
        NsStep::DbForm => db_form(ns),
        NsStep::ComposeForm => compose_form(ns),
        NsStep::PickTemplate => pick_template(ns),
        NsStep::TemplateForm => template_form(ns),
    };

    column![
        row![
            text("Novo Serviço").size(20).color(palette::CYAN),
            Space::with_width(Length::Fill),
            ghost_btn("‹ Voltar", Message::NsBack),
            ghost_btn("Cancelar", Message::NsCancel),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        Space::with_height(Length::Fixed(12.0)),
        body,
    ]
    .spacing(4)
    .width(Length::Fixed(680.0))
    .into()
}

fn pick_type() -> Element<'static, Message> {
    let mut col = column![text("Escolha o tipo de serviço").size(14).color(palette::GRAY)].spacing(8);
    for kind in ServiceKind::ALL {
        col = col.push(
            button(
                column![
                    text(kind.label().to_string()).size(15).color(palette::CYAN),
                    text(kind.description().to_string()).size(12).color(palette::GRAY),
                ]
                .spacing(2),
            )
            .on_press(Message::NsPickType(*kind))
            .width(Length::Fill)
            .style(button::secondary)
            .padding([8, 12]),
        );
    }
    col.into()
}

fn pick_db() -> Element<'static, Message> {
    let mut col = column![text("Escolha o banco de dados").size(14).color(palette::GRAY)].spacing(6);
    for db in DbKind::ALL {
        col = col.push(
            button(
                row![
                    text(db.label().to_string()).size(14).color(palette::CYAN).width(Length::Fixed(140.0)),
                    text(db.default_image().to_string()).size(12).color(palette::GRAY),
                ]
                .spacing(8),
            )
            .on_press(Message::NsPickDb(*db))
            .width(Length::Fill)
            .style(button::secondary)
            .padding([8, 12]),
        );
    }
    col.into()
}

fn nsf<'a>(label: &'a str, placeholder: &'a str, value: &'a str, field: NsField) -> Element<'a, Message> {
    labeled_input(label, placeholder, value, move |v| Message::NsField(field, v))
}

fn app_form(ns: &NsForm) -> Element<'_, Message> {
    column![
        section("Nova Application"),
        nsf("Nome", "minha-app", &ns.name, NsField::Name),
        nsf("App Name", "(opcional)", &ns.app_name, NsField::AppName),
        nsf("Descrição", "(opcional)", &ns.description, NsField::Description),
        Space::with_height(Length::Fixed(10.0)),
        primary_btn("Criar Application", Message::NsCreate),
    ]
    .spacing(6)
    .into()
}

fn compose_form(ns: &NsForm) -> Element<'_, Message> {
    column![
        section("Nova Compose Stack"),
        nsf("Nome", "minha-stack", &ns.name, NsField::Name),
        nsf("App Name", "(opcional)", &ns.app_name, NsField::AppName),
        muted("Configure o compose file dentro do serviço antes de fazer deploy."),
        Space::with_height(Length::Fixed(10.0)),
        primary_btn("Criar Compose Stack", Message::NsCreate),
    ]
    .spacing(6)
    .into()
}

fn db_form(ns: &NsForm) -> Element<'_, Message> {
    let db = match ns.db_kind {
        Some(d) => d,
        None => return text("").into(),
    };
    let mut col = column![
        section(&format!("Nova {}", db.label())),
        nsf("Nome", "meu-banco", &ns.name, NsField::Name),
        nsf("App Name", "(opcional)", &ns.app_name, NsField::AppName),
        nsf("Descrição", "(opcional)", &ns.description, NsField::Description),
    ]
    .spacing(6);

    match db {
        DbKind::Postgres => {
            col = col.push(nsf("Database Name", "app", &ns.db_name, NsField::DbName));
            col = col.push(nsf("User", "postgres", &ns.db_user, NsField::DbUser));
            col = col.push(nsf("Password", "", &ns.db_password, NsField::DbPassword));
        }
        DbKind::MongoDB => {
            col = col.push(nsf("User", "root", &ns.db_user, NsField::DbUser));
            col = col.push(nsf("Password", "", &ns.db_password, NsField::DbPassword));
            col = col.push(
                row![
                    iced::widget::container(label_text("Use Replica Sets")).width(Length::Fixed(190.0)),
                    checkbox("", ns.use_replica_sets).on_toggle(Message::NsReplica),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        DbKind::MariaDB | DbKind::MySQL => {
            col = col.push(nsf("Database Name", "app", &ns.db_name, NsField::DbName));
            col = col.push(nsf("User", "user", &ns.db_user, NsField::DbUser));
            col = col.push(nsf("Password", "", &ns.db_password, NsField::DbPassword));
            col = col.push(nsf("Root Password", "", &ns.db_root_password, NsField::DbRootPassword));
        }
        DbKind::Redis => {
            col = col.push(nsf("Password", "(opcional)", &ns.db_password, NsField::DbPassword));
        }
    }
    col = col.push(nsf("Docker Image", db.default_image(), &ns.docker_image, NsField::Image));
    col = col.push(Space::with_height(Length::Fixed(10.0)));
    col = col.push(primary_btn(&format!("Criar {}", db.label()), Message::NsCreate));
    scrollable(col).height(Length::Fixed(420.0)).into()
}

fn pick_template(ns: &NsForm) -> Element<'_, Message> {
    // Category filters
    let mut cats = row![].spacing(4);
    for (i, cat) in TemplateCategory::FILTERS.iter().enumerate() {
        let active = i == ns.template_cat;
        cats = cats.push(
            button(text(cat.label().to_string()).size(12))
                .on_press(Message::NsTemplateCat(i))
                .style(if active { button::primary } else { button::text })
                .padding([3, 8]),
        );
    }

    let cat = TemplateCategory::FILTERS[ns.template_cat.min(TemplateCategory::FILTERS.len() - 1)];
    let list = templates::filtered(cat, &ns.template_search);

    let mut items = column![].spacing(3);
    if list.is_empty() {
        items = items.push(muted("Nenhum template encontrado."));
    } else {
        for t in list {
            items = items.push(
                button(
                    row![
                        text(t.name.to_string()).size(13).color(palette::CYAN).width(Length::Fixed(180.0)),
                        text(format!("[{}]", t.category.label())).size(11).color(palette::GRAY).width(Length::Fixed(130.0)),
                        text(t.description.to_string()).size(12).color(palette::GRAY),
                    ]
                    .spacing(8),
                )
                .on_press(Message::NsTemplateSelect(t.id))
                .width(Length::Fill)
                .style(button::secondary)
                .padding([5, 10]),
            );
        }
    }

    column![
        scrollable(cats),
        Space::with_height(Length::Fixed(6.0)),
        text_input("buscar templates…", &ns.template_search)
            .on_input(Message::NsTemplateSearch)
            .padding(6)
            .size(13),
        Space::with_height(Length::Fixed(6.0)),
        scrollable(items).height(Length::Fixed(380.0)),
    ]
    .spacing(4)
    .into()
}

fn template_form(ns: &NsForm) -> Element<'_, Message> {
    let Some(t) = ns.selected_template else {
        return text("").into();
    };
    let mut col = column![
        section(&format!("{} — Configurar", t.name)),
        nsf("Nome do serviço", t.name, &ns.name, NsField::Name),
    ]
    .spacing(6);

    for (i, var) in t.variables.iter().enumerate() {
        let val = ns.template_var_values.get(i).map(String::as_str).unwrap_or("");
        let placeholder = var.default.unwrap_or("");
        col = col.push(template_var_input(var.label, placeholder, val, i));
    }
    col = col.push(Space::with_height(Length::Fixed(10.0)));
    col = col.push(primary_btn(&format!("Criar {}", t.name), Message::NsCreate));
    scrollable(col).height(Length::Fixed(420.0)).into()
}

fn template_var_input<'a>(
    label: &'a str,
    placeholder: &'a str,
    value: &'a str,
    idx: usize,
) -> Element<'a, Message> {
    row![
        iced::widget::container(text(label.to_string()).size(13).color(palette::GRAY)).width(Length::Fixed(220.0)),
        text_input(placeholder, value).on_input(move |v| Message::NsTemplateVar(idx, v)).padding(6).size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}
